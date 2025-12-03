//! MPEG-2 TS SI (Service Information) parser
//!
//! Parses PAT, PMT, SDT, NIT tables from MPEG-2 Transport Stream
//! to extract service information.

use std::collections::HashMap;

use crate::error::Result;

/// TS packet size
pub const TS_PACKET_SIZE: usize = 188;

/// TS sync byte
pub const TS_SYNC_BYTE: u8 = 0x47;

/// PAT PID
pub const PAT_PID: u16 = 0x0000;

/// SDT PID
pub const SDT_PID: u16 = 0x0011;

/// NIT PID
pub const NIT_PID: u16 = 0x0010;

/// Table ID for PAT
pub const TABLE_ID_PAT: u8 = 0x00;

/// Table ID for PMT
pub const TABLE_ID_PMT: u8 = 0x02;

/// Table ID for SDT (actual)
pub const TABLE_ID_SDT_ACTUAL: u8 = 0x42;

/// Table ID for NIT (actual)
pub const TABLE_ID_NIT_ACTUAL: u8 = 0x40;

/// Parsed service information
#[derive(Debug, Clone, Default)]
pub struct ServiceInfo {
    /// Service ID (program_number)
    pub service_id: u16,
    /// PMT PID
    pub pmt_pid: Option<u16>,
    /// Service name from SDT
    pub service_name: Option<String>,
    /// Service provider (broadcaster) name from SDT
    pub provider_name: Option<String>,
    /// Network ID from NIT
    pub network_id: Option<u16>,
    /// Transport stream ID
    pub transport_stream_id: Option<u16>,
}

/// SI parser for extracting service information from TS stream
#[derive(Debug, Default)]
pub struct SiParser {
    /// PAT data: program_number -> PMT PID
    pat_programs: HashMap<u16, u16>,
    /// Transport stream ID from PAT
    transport_stream_id: Option<u16>,
    /// SDT data: service_id -> (service_name, provider_name)
    sdt_services: HashMap<u16, (Option<String>, Option<String>)>,
    /// Network ID from NIT
    network_id: Option<u16>,
    /// Collected services
    services: Vec<ServiceInfo>,
    /// Whether PAT has been parsed
    pat_parsed: bool,
    /// SDT section tracking: (last_section_number, set of received sections)
    sdt_sections: Option<(u8, std::collections::HashSet<u8>)>,
}

impl SiParser {
    /// Create a new SI parser
    pub fn new() -> Self {
        SiParser::default()
    }

    /// Find TS sync pattern and determine packet size
    /// Returns (start_offset, packet_size)
    fn find_sync_pattern(&self, data: &[u8]) -> (Option<usize>, Option<usize>) {
        // Common packet sizes: 188 (standard), 192 (with timestamp), 204 (with RS coding)
        let packet_sizes = [188, 192, 204];

        // Search in first 1024 bytes for sync pattern
        let search_len = std::cmp::min(1024, data.len());

        for i in 0..search_len {
            if data[i] == TS_SYNC_BYTE {
                // Found potential sync byte, verify with next packets
                for &pkt_size in &packet_sizes {
                    let mut valid = true;
                    // Check next 3 sync bytes to confirm pattern
                    for j in 1..=3 {
                        let next_pos = i + j * pkt_size;
                        if next_pos >= data.len() {
                            // Not enough data to verify, assume this size might work
                            if j >= 2 {
                                // At least 2 sync bytes found
                                return (Some(i), Some(pkt_size));
                            }
                            valid = false;
                            break;
                        }
                        if data[next_pos] != TS_SYNC_BYTE {
                            valid = false;
                            break;
                        }
                    }
                    if valid {
                        return (Some(i), Some(pkt_size));
                    }
                }
            }
        }

        (None, None)
    }

    /// Parse TS data and extract SI information
    pub fn parse(&mut self, ts_data: &[u8]) -> Result<()> {
        // Debug: Log first 32 bytes of data to diagnose format issues
        if ts_data.len() >= 32 {
            log::debug!(
                "First 32 bytes: {:02X?}",
                &ts_data[..32]
            );
        }

        // Try to find sync byte and determine packet size
        let (start_offset, packet_size) = self.find_sync_pattern(ts_data);

        if start_offset.is_none() {
            log::debug!("No TS sync pattern found in {} bytes of data", ts_data.len());
            return Ok(());
        }

        let start_offset = start_offset.unwrap();
        let packet_size = packet_size.unwrap_or(TS_PACKET_SIZE);

        log::debug!(
            "Found TS sync at offset {}, packet size {} bytes",
            start_offset,
            packet_size
        );

        // Process each TS packet
        let mut offset = start_offset;
        let mut sync_errors = 0;
        let mut packets_processed = 0;
        let mut pat_packets = 0;
        let mut sdt_packets = 0;
        let mut nit_packets = 0;

        // Use standard 188 byte packet for actual TS data parsing
        // (packet_size includes any prefix bytes like timestamp)
        let ts_packet_len = TS_PACKET_SIZE;
        let prefix_len = packet_size - ts_packet_len;

        while offset + packet_size <= ts_data.len() {
            // Skip any prefix bytes (timestamp, etc.) to get to actual TS packet
            let packet_start = offset + prefix_len;
            let packet = &ts_data[packet_start..packet_start + ts_packet_len];

            if packet[0] != TS_SYNC_BYTE {
                // Try to find sync byte
                sync_errors += 1;
                offset += 1;
                continue;
            }

            packets_processed += 1;

            // Extract PID
            let pid = ((packet[1] as u16 & 0x1F) << 8) | packet[2] as u16;

            // Check for payload unit start indicator
            let payload_unit_start = (packet[1] & 0x40) != 0;

            // Get adaptation field control
            let adaptation_field_control = (packet[3] >> 4) & 0x03;

            // Calculate payload offset
            let mut payload_offset = 4;
            if adaptation_field_control == 2 || adaptation_field_control == 3 {
                // Adaptation field present
                let adaptation_length = packet[4] as usize;
                payload_offset = 5 + adaptation_length;
            }

            if payload_offset >= ts_packet_len {
                offset += packet_size;
                continue;
            }

            // Only process packets with payload
            if adaptation_field_control == 0 || adaptation_field_control == 2 {
                offset += packet_size;
                continue;
            }

            // Process payload based on PID
            if payload_unit_start && payload_offset < ts_packet_len {
                let payload = &packet[payload_offset..];

                match pid {
                    PAT_PID => {
                        pat_packets += 1;
                        if !self.pat_parsed {
                            self.parse_pat_section(payload)?;
                        }
                    }
                    SDT_PID => {
                        sdt_packets += 1;
                        // Always try to parse SDT - we need all sections
                        self.parse_sdt_section(payload)?;
                    }
                    NIT_PID => {
                        nit_packets += 1;
                        self.parse_nit_section(payload)?;
                    }
                    _ => {}
                }
            }

            offset += packet_size;
        }

        log::debug!(
            "Parsed {} packets (sync_errors={}, PAT={}, SDT={}, NIT={})",
            packets_processed,
            sync_errors,
            pat_packets,
            sdt_packets,
            nit_packets
        );

        Ok(())
    }

    /// Parse PAT (Program Association Table) section
    fn parse_pat_section(&mut self, data: &[u8]) -> Result<()> {
        if data.len() < 9 {
            return Ok(());
        }

        // Skip pointer field if present
        let pointer_field = data[0] as usize;
        let section_start = 1 + pointer_field;

        if section_start >= data.len() {
            return Ok(());
        }

        let section = &data[section_start..];
        if section.is_empty() || section[0] != TABLE_ID_PAT {
            return Ok(());
        }

        // Section length
        let section_length = ((section[1] as usize & 0x0F) << 8) | section[2] as usize;
        if section.len() < 3 + section_length || section_length < 9 {
            return Ok(());
        }

        // Transport stream ID
        self.transport_stream_id = Some(((section[3] as u16) << 8) | section[4] as u16);

        // Parse program loop
        let loop_start = 8;
        let loop_end = 3 + section_length - 4; // Exclude CRC

        let mut pos = loop_start;
        while pos + 4 <= loop_end && pos + 4 <= section.len() {
            let program_number = ((section[pos] as u16) << 8) | section[pos + 1] as u16;
            let pmt_pid = ((section[pos + 2] as u16 & 0x1F) << 8) | section[pos + 3] as u16;

            if program_number != 0 {
                // Skip NIT entry (program_number = 0)
                self.pat_programs.insert(program_number, pmt_pid);
            }

            pos += 4;
        }

        self.pat_parsed = true;
        log::debug!(
            "PAT parsed: {} programs, TSID={}",
            self.pat_programs.len(),
            self.transport_stream_id.unwrap_or(0)
        );

        Ok(())
    }

    /// Parse SDT (Service Description Table) section
    fn parse_sdt_section(&mut self, data: &[u8]) -> Result<()> {
        if data.len() < 12 {
            return Ok(());
        }

        // Skip pointer field if present
        let pointer_field = data[0] as usize;
        let section_start = 1 + pointer_field;

        if section_start >= data.len() {
            return Ok(());
        }

        let section = &data[section_start..];
        if section.is_empty() || section[0] != TABLE_ID_SDT_ACTUAL {
            return Ok(());
        }

        // Section length
        let section_length = ((section[1] as usize & 0x0F) << 8) | section[2] as usize;
        if section.len() < 3 + section_length || section_length < 12 {
            return Ok(());
        }

        // Get section_number and last_section_number
        let section_number = section[6];
        let last_section_number = section[7];

        // Check if we've already processed this section
        if let Some((_, ref received)) = self.sdt_sections {
            if received.contains(&section_number) {
                return Ok(());
            }
        }

        // Parse service loop
        let loop_start = 11;
        let loop_end = 3 + section_length - 4; // Exclude CRC

        let mut pos = loop_start;
        let mut services_in_section = 0;
        while pos + 5 <= loop_end && pos + 5 <= section.len() {
            let service_id = ((section[pos] as u16) << 8) | section[pos + 1] as u16;
            let descriptors_loop_length =
                ((section[pos + 3] as usize & 0x0F) << 8) | section[pos + 4] as usize;

            pos += 5;

            // Parse descriptors
            let desc_end = pos + descriptors_loop_length;
            while pos + 2 <= desc_end && pos + 2 <= section.len() {
                let descriptor_tag = section[pos];
                let descriptor_length = section[pos + 1] as usize;

                if descriptor_tag == 0x48 && pos + 2 + descriptor_length <= section.len() {
                    // Service descriptor
                    let desc_data = &section[pos + 2..pos + 2 + descriptor_length];
                    if let Some((provider, service)) = self.parse_service_descriptor(desc_data) {
                        self.sdt_services.insert(service_id, (service, provider));
                        services_in_section += 1;
                    }
                }

                pos += 2 + descriptor_length;
            }

            pos = desc_end;
        }

        // Track this section
        match &mut self.sdt_sections {
            Some((_, ref mut received)) => {
                received.insert(section_number);
            }
            None => {
                let mut received = std::collections::HashSet::new();
                received.insert(section_number);
                self.sdt_sections = Some((last_section_number, received));
            }
        }

        log::debug!(
            "SDT section {}/{} parsed: {} services in section, {} total",
            section_number,
            last_section_number,
            services_in_section,
            self.sdt_services.len()
        );

        Ok(())
    }

    /// Check if all SDT sections have been received
    fn sdt_complete(&self) -> bool {
        if let Some((last_section, ref received)) = self.sdt_sections {
            // Check if we have all sections from 0 to last_section_number
            for i in 0..=last_section {
                if !received.contains(&i) {
                    return false;
                }
            }
            true
        } else {
            false
        }
    }

    /// Parse NIT (Network Information Table) section
    fn parse_nit_section(&mut self, data: &[u8]) -> Result<()> {
        if data.len() < 10 {
            return Ok(());
        }

        // Skip pointer field if present
        let pointer_field = data[0] as usize;
        let section_start = 1 + pointer_field;

        if section_start >= data.len() {
            return Ok(());
        }

        let section = &data[section_start..];
        if section.is_empty() || section[0] != TABLE_ID_NIT_ACTUAL {
            return Ok(());
        }

        // Network ID
        if section.len() >= 5 {
            self.network_id = Some(((section[3] as u16) << 8) | section[4] as u16);
            log::debug!("NIT parsed: network_id={}", self.network_id.unwrap_or(0));
        }

        Ok(())
    }

    /// Parse service descriptor (0x48)
    fn parse_service_descriptor(&self, data: &[u8]) -> Option<(Option<String>, Option<String>)> {
        if data.len() < 3 {
            return None;
        }

        let _service_type = data[0];
        let provider_name_length = data[1] as usize;

        if data.len() < 2 + provider_name_length + 1 {
            return None;
        }

        let provider_name = if provider_name_length > 0 {
            Some(decode_arib_string(&data[2..2 + provider_name_length]))
        } else {
            None
        };

        let service_name_offset = 2 + provider_name_length;
        let service_name_length = data[service_name_offset] as usize;

        let service_name = if service_name_length > 0
            && service_name_offset + 1 + service_name_length <= data.len()
        {
            Some(decode_arib_string(
                &data[service_name_offset + 1..service_name_offset + 1 + service_name_length],
            ))
        } else {
            None
        };

        Some((provider_name, service_name))
    }

    /// Get collected services
    pub fn get_services(&self) -> Vec<ServiceInfo> {
        let mut services = Vec::new();

        for (&program_number, &pmt_pid) in &self.pat_programs {
            let mut info = ServiceInfo {
                service_id: program_number,
                pmt_pid: Some(pmt_pid),
                network_id: self.network_id,
                transport_stream_id: self.transport_stream_id,
                ..Default::default()
            };

            if let Some((service_name, provider_name)) = self.sdt_services.get(&program_number) {
                info.service_name = service_name.clone();
                info.provider_name = provider_name.clone();
            }

            services.push(info);
        }

        services
    }

    /// Check if basic SI information has been collected
    pub fn has_basic_info(&self) -> bool {
        self.pat_parsed && !self.pat_programs.is_empty()
    }

    /// Check if service names have been collected for all services
    pub fn has_service_names(&self) -> bool {
        // Need to have received at least one SDT section
        if self.sdt_sections.is_none() {
            return false;
        }

        // Check if we have service names for all services in PAT
        // (Some data services may not have names, so we use a threshold)
        if self.pat_programs.is_empty() {
            return false;
        }

        let services_with_names = self.pat_programs.keys()
            .filter(|&id| self.sdt_services.contains_key(id))
            .count();

        // Consider complete if we have names for at least 50% of services
        // or if all SDT sections have been received
        let coverage = services_with_names as f64 / self.pat_programs.len() as f64;
        coverage >= 0.5 || self.sdt_complete()
    }

    /// Reset parser state
    pub fn reset(&mut self) {
        self.pat_programs.clear();
        self.transport_stream_id = None;
        self.sdt_services.clear();
        self.network_id = None;
        self.services.clear();
        self.pat_parsed = false;
        self.sdt_sections = None;
    }
}

/// ARIB STD-B24 character decoder
///
/// Decodes Japanese digital TV character strings according to ARIB STD-B24.
/// Handles JIS X 0208 Kanji, Hiragana, Katakana, and alphanumeric characters.
pub struct AribDecoder {
    /// Current GL (left) character set: 0=G0, 1=G1, 2=G2, 3=G3
    gl: u8,
    /// Current GR (right) character set
    gr: u8,
    /// Single shift mode: None, Some(2)=SS2, Some(3)=SS3
    single_shift: Option<u8>,
    /// G0-G3 character set designations
    /// 0=Kanji, 1=Alphanumeric, 2=Hiragana, 3=Katakana, 4=JIS_X_0201_Katakana
    g_sets: [u8; 4],
}

impl Default for AribDecoder {
    fn default() -> Self {
        // Default designation for Japanese broadcasting
        AribDecoder {
            gl: 0,  // GL = G0
            gr: 2,  // GR = G2
            single_shift: None,
            g_sets: [
                0,  // G0 = Kanji (JIS X 0208)
                1,  // G1 = Alphanumeric
                2,  // G2 = Hiragana
                3,  // G3 = Katakana
            ],
        }
    }
}

impl AribDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Decode ARIB STD-B24 string
    pub fn decode(&mut self, data: &[u8]) -> String {
        let mut result = String::new();
        let mut i = 0;

        while i < data.len() {
            let b = data[i];

            // Handle control codes
            if b < 0x20 {
                match b {
                    0x0F => self.gl = 0,  // LS0: GL = G0
                    0x0E => self.gl = 1,  // LS1: GL = G1
                    0x19 => self.single_shift = Some(2),  // SS2
                    0x1D => self.single_shift = Some(3),  // SS3
                    0x1B => {
                        // ESC sequence
                        i = self.parse_escape_sequence(data, i);
                        continue;
                    }
                    0x0A | 0x0D => {
                        // Keep newlines
                        result.push(b as char);
                    }
                    0x20 => {
                        result.push(' ');
                    }
                    _ => {}  // Skip other control codes
                }
                i += 1;
                continue;
            }

            // Handle space (0x20)
            if b == 0x20 {
                result.push(' ');
                i += 1;
                continue;
            }

            // Determine which character set to use
            let g_set = if let Some(ss) = self.single_shift.take() {
                self.g_sets[ss as usize]
            } else if b >= 0x80 {
                // GR area (0x80-0xFF) - use GR character set
                self.g_sets[self.gr as usize]
            } else {
                // GL area (0x21-0x7E) - use GL character set
                self.g_sets[self.gl as usize]
            };

            // Decode based on character set
            match g_set {
                0 => {
                    // Kanji (JIS X 0208) - 2 bytes
                    if i + 1 < data.len() {
                        let b1 = (b & 0x7F) as u16;
                        let b2 = (data[i + 1] & 0x7F) as u16;
                        if let Some(c) = jis_x_0208_to_unicode(b1, b2) {
                            result.push(c);
                        }
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                1 => {
                    // Alphanumeric
                    let c = b & 0x7F;
                    if (0x21..=0x7E).contains(&c) {
                        result.push(c as char);
                    }
                    i += 1;
                }
                2 => {
                    // Hiragana
                    let c = b & 0x7F;
                    if (0x21..=0x7E).contains(&c) {
                        // Map to Unicode Hiragana block (U+3041 - U+3096)
                        let unicode = 0x3040 + (c as u32 - 0x20);
                        if let Some(ch) = char::from_u32(unicode) {
                            result.push(ch);
                        }
                    }
                    i += 1;
                }
                3 => {
                    // Katakana
                    let c = b & 0x7F;
                    if (0x21..=0x7E).contains(&c) {
                        // Map to Unicode Katakana block (U+30A1 - U+30F6)
                        let unicode = 0x30A0 + (c as u32 - 0x20);
                        if let Some(ch) = char::from_u32(unicode) {
                            result.push(ch);
                        }
                    }
                    i += 1;
                }
                4 => {
                    // JIS X 0201 Katakana (half-width)
                    let c = b & 0x7F;
                    if (0x21..=0x5F).contains(&c) {
                        let unicode = 0xFF60 + (c as u32 - 0x20);
                        if let Some(ch) = char::from_u32(unicode) {
                            result.push(ch);
                        }
                    }
                    i += 1;
                }
                _ => {
                    i += 1;
                }
            }
        }

        result
    }

    /// Parse escape sequence and update state
    fn parse_escape_sequence(&mut self, data: &[u8], pos: usize) -> usize {
        if pos + 1 >= data.len() {
            return pos + 1;
        }

        let b1 = data[pos + 1];

        match b1 {
            0x24 => {
                // 2-byte character set designation
                if pos + 2 >= data.len() {
                    return pos + 2;
                }
                let b2 = data[pos + 2];
                match b2 {
                    0x28 | 0x29 | 0x2A | 0x2B => {
                        // G0-G3 designation with intermediate byte
                        if pos + 3 >= data.len() {
                            return pos + 3;
                        }
                        let g_num = (b2 - 0x28) as usize;
                        let final_byte = data[pos + 3];
                        self.g_sets[g_num] = self.final_byte_to_charset(final_byte, true);
                        return pos + 4;
                    }
                    0x42 | 0x40 => {
                        // Designate JIS X 0208 to G0
                        self.g_sets[0] = 0;  // Kanji
                        return pos + 3;
                    }
                    _ => {
                        return pos + 3;
                    }
                }
            }
            0x28 | 0x29 | 0x2A | 0x2B => {
                // 1-byte character set designation to G0-G3
                if pos + 2 >= data.len() {
                    return pos + 2;
                }
                let g_num = (b1 - 0x28) as usize;
                let final_byte = data[pos + 2];
                self.g_sets[g_num] = self.final_byte_to_charset(final_byte, false);
                return pos + 3;
            }
            0x6E => {
                // LS2: GL = G2
                self.gl = 2;
                return pos + 2;
            }
            0x6F => {
                // LS3: GL = G3
                self.gl = 3;
                return pos + 2;
            }
            0x7E => {
                // LS1R: GR = G1
                self.gr = 1;
                return pos + 2;
            }
            0x7D => {
                // LS2R: GR = G2
                self.gr = 2;
                return pos + 2;
            }
            0x7C => {
                // LS3R: GR = G3
                self.gr = 3;
                return pos + 2;
            }
            _ => {
                return pos + 2;
            }
        }
    }

    /// Convert final byte to character set ID
    fn final_byte_to_charset(&self, b: u8, is_2byte: bool) -> u8 {
        if is_2byte {
            match b {
                0x42 | 0x40 => 0,  // JIS X 0208 Kanji
                0x39 | 0x3B => 0,  // JIS X 0213 Kanji
                _ => 0,
            }
        } else {
            match b {
                0x42 => 1,  // ASCII / Alphanumeric
                0x4A => 1,  // JIS X 0201 Roman
                0x30 => 2,  // Hiragana
                0x31 => 3,  // Katakana
                0x32 => 4,  // Mosaic A
                0x33 => 4,  // Mosaic B
                0x34 => 4,  // Mosaic C
                0x35 => 4,  // Mosaic D
                0x36 => 1,  // Proportional alphanumeric
                0x37 => 2,  // Proportional hiragana
                0x38 => 3,  // Proportional katakana
                0x49 => 4,  // JIS X 0201 Katakana
                _ => 1,     // Default to alphanumeric
            }
        }
    }
}

/// Convert JIS X 0208 code to Unicode
fn jis_x_0208_to_unicode(row: u16, cell: u16) -> Option<char> {
    // JIS X 0208 is organized in rows (ku) and cells (ten)
    // Row 1-8: Symbols and special characters
    // Row 16-47: Level 1 Kanji
    // Row 48-84: Level 2 Kanji

    if row < 0x21 || row > 0x7E || cell < 0x21 || cell > 0x7E {
        return None;
    }

    // Convert to Shift_JIS then decode
    // JIS to Shift_JIS conversion
    let jis_row = row - 0x21;
    let jis_cell = cell - 0x21;

    let sjis_hi = if jis_row < 63 {
        ((jis_row / 2) + 0x81) as u8
    } else {
        ((jis_row / 2) + 0xC1) as u8
    };

    let sjis_lo = if jis_row % 2 == 0 {
        if jis_cell < 63 {
            (jis_cell + 0x40) as u8
        } else {
            (jis_cell + 0x41) as u8
        }
    } else {
        (jis_cell + 0x9F) as u8
    };

    // Decode Shift_JIS
    let sjis_bytes = [sjis_hi, sjis_lo];
    let (decoded, _, had_errors) = encoding_rs::SHIFT_JIS.decode(&sjis_bytes);
    if !had_errors {
        decoded.chars().next()
    } else {
        None
    }
}

/// Decode ARIB STD-B24 character string
pub fn decode_arib_string(data: &[u8]) -> String {
    if data.is_empty() {
        return String::new();
    }

    let mut decoder = AribDecoder::new();
    decoder.decode(data)
}
