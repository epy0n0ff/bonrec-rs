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
    /// Whether SDT has been parsed
    sdt_parsed: bool,
}

impl SiParser {
    /// Create a new SI parser
    pub fn new() -> Self {
        SiParser::default()
    }

    /// Parse TS data and extract SI information
    pub fn parse(&mut self, ts_data: &[u8]) -> Result<()> {
        // Process each TS packet
        let mut offset = 0;

        while offset + TS_PACKET_SIZE <= ts_data.len() {
            let packet = &ts_data[offset..offset + TS_PACKET_SIZE];

            if packet[0] != TS_SYNC_BYTE {
                // Try to find sync byte
                offset += 1;
                continue;
            }

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

            if payload_offset >= TS_PACKET_SIZE {
                offset += TS_PACKET_SIZE;
                continue;
            }

            // Only process packets with payload
            if adaptation_field_control == 0 || adaptation_field_control == 2 {
                offset += TS_PACKET_SIZE;
                continue;
            }

            // Process payload based on PID
            if payload_unit_start && payload_offset < TS_PACKET_SIZE {
                let payload = &packet[payload_offset..];

                match pid {
                    PAT_PID => {
                        if !self.pat_parsed {
                            self.parse_pat_section(payload)?;
                        }
                    }
                    SDT_PID => {
                        if !self.sdt_parsed {
                            self.parse_sdt_section(payload)?;
                        }
                    }
                    NIT_PID => {
                        self.parse_nit_section(payload)?;
                    }
                    _ => {}
                }
            }

            offset += TS_PACKET_SIZE;
        }

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

        // Parse service loop
        let loop_start = 11;
        let loop_end = 3 + section_length - 4; // Exclude CRC

        let mut pos = loop_start;
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
                    }
                }

                pos += 2 + descriptor_length;
            }

            pos = desc_end;
        }

        self.sdt_parsed = true;
        log::debug!("SDT parsed: {} services", self.sdt_services.len());

        Ok(())
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

    /// Check if service names have been collected
    pub fn has_service_names(&self) -> bool {
        self.sdt_parsed && !self.sdt_services.is_empty()
    }

    /// Reset parser state
    pub fn reset(&mut self) {
        self.pat_programs.clear();
        self.transport_stream_id = None;
        self.sdt_services.clear();
        self.network_id = None;
        self.services.clear();
        self.pat_parsed = false;
        self.sdt_parsed = false;
    }
}

/// Decode ARIB STD-B24 character string
///
/// This is a simplified decoder that handles common cases.
/// For full ARIB support, a dedicated library would be needed.
pub fn decode_arib_string(data: &[u8]) -> String {
    if data.is_empty() {
        return String::new();
    }

    // Check for common character set designators
    // 0x1B is ESC for character set switching
    // For simplicity, we try to decode as UTF-8 or Shift_JIS

    // Try to find and skip ESC sequences
    let mut clean_data = Vec::with_capacity(data.len());
    let mut i = 0;

    while i < data.len() {
        if data[i] == 0x1B && i + 2 < data.len() {
            // Skip ESC sequence (typically 3 bytes)
            i += 3;
            continue;
        }

        // Skip control characters except common ones
        if data[i] < 0x20 && data[i] != 0x0A && data[i] != 0x0D {
            i += 1;
            continue;
        }

        clean_data.push(data[i]);
        i += 1;
    }

    // Try Shift_JIS decoding first (common for Japanese broadcasting)
    let (decoded, _, had_errors) = encoding_rs::SHIFT_JIS.decode(&clean_data);
    if !had_errors {
        return decoded.into_owned();
    }

    // Fallback to UTF-8
    String::from_utf8_lossy(&clean_data).into_owned()
}
