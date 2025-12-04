//! Mirakurun channels.yml output formatting

use serde::Serialize;
use std::io::Write;

use crate::error::Result;
use crate::scanner::ScannedChannelInfo;
use crate::storage::ExportedChannel;

/// Mirakurun channel entry
#[derive(Debug, Clone, Serialize)]
pub struct MirakurunChannel {
    /// Service/Channel name
    pub name: String,
    /// Channel type: GR, BS, CS
    #[serde(rename = "type")]
    pub channel_type: String,
    /// Physical channel number (as string)
    pub channel: String,
    /// Tuning space index (for BS/CS with BonDriver)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub space: Option<u32>,
    /// Service ID (optional)
    #[serde(rename = "serviceId", skip_serializing_if = "Option::is_none")]
    pub service_id: Option<u16>,
}

/// Detect channel type from tuning space name
fn detect_channel_type(tuning_space: &str) -> &'static str {
    let space_lower = tuning_space.to_lowercase();
    if space_lower.contains("bs") {
        "BS"
    } else if space_lower.contains("cs") {
        "CS"
    } else {
        // Default to GR (terrestrial) for 地デジ, UHF, VHF, etc.
        "GR"
    }
}

/// Check if channel type needs space field
fn needs_space(channel_type: &str) -> bool {
    channel_type == "BS" || channel_type == "CS"
}

/// Get sort order for channel type
fn channel_type_order(channel_type: &str) -> u8 {
    match channel_type {
        "GR" => 0,
        "BS" => 1,
        "CS" => 2,
        _ => 3,
    }
}

/// Sort Mirakurun channels by type, channel number, and service ID
fn sort_channels(channels: &mut [MirakurunChannel]) {
    channels.sort_by(|a, b| {
        // First, sort by channel type (GR < BS < CS)
        let type_cmp = channel_type_order(&a.channel_type)
            .cmp(&channel_type_order(&b.channel_type));
        if type_cmp != std::cmp::Ordering::Equal {
            return type_cmp;
        }

        // Then, sort by channel number (numeric)
        let a_ch: u32 = a.channel.parse().unwrap_or(u32::MAX);
        let b_ch: u32 = b.channel.parse().unwrap_or(u32::MAX);
        let ch_cmp = a_ch.cmp(&b_ch);
        if ch_cmp != std::cmp::Ordering::Equal {
            return ch_cmp;
        }

        // Finally, sort by service ID
        a.service_id.cmp(&b.service_id)
    });
}

/// Convert scanned channel info to Mirakurun channel entries
pub fn to_mirakurun_channels(channels: &[ScannedChannelInfo]) -> Vec<MirakurunChannel> {
    let mut result = Vec::new();

    for channel in channels {
        let channel_type = detect_channel_type(&channel.tuning_space);
        // For BS/CS, use channel_index (numeric index for BonDriver tuning)
        // For GR, use physical channel number
        let channel_value = if needs_space(channel_type) {
            channel.channel_index.to_string()
        } else {
            channel
                .physical_channel
                .map(|c| c.to_string())
                .or_else(|| channel.channel_name.clone())
                .unwrap_or_else(|| channel.channel_index.to_string())
        };

        // Include space for BS/CS channels
        let space = if needs_space(channel_type) {
            Some(channel.space_index)
        } else {
            None
        };

        // Create an entry for each service
        for service in &channel.services {
            let name = service
                .service_name
                .clone()
                .or_else(|| channel.channel_name.clone())
                .unwrap_or_else(|| format!("Service {}", service.service_id));

            result.push(MirakurunChannel {
                name,
                channel_type: channel_type.to_string(),
                channel: channel_value.clone(),
                space,
                service_id: Some(service.service_id),
            });
        }
    }

    sort_channels(&mut result);
    result
}

/// Convert exported channels to Mirakurun channel entries
pub fn exported_to_mirakurun_channels(channels: &[ExportedChannel]) -> Vec<MirakurunChannel> {
    let mut result = Vec::new();

    for channel in channels {
        let channel_type = detect_channel_type(&channel.tuning_space);
        // For BS/CS, use channel_index (numeric index for BonDriver tuning)
        // For GR, use physical channel number
        let channel_value = if needs_space(channel_type) {
            channel.channel_index.to_string()
        } else {
            channel
                .physical_channel
                .map(|c| c.to_string())
                .or_else(|| channel.channel_name.clone())
                .unwrap_or_else(|| channel.channel_index.to_string())
        };

        // Include space for BS/CS channels
        let space = if needs_space(channel_type) {
            Some(channel.space_index)
        } else {
            None
        };

        // Create an entry for each service
        for service in &channel.services {
            let name = service
                .service_name
                .clone()
                .or_else(|| channel.channel_name.clone())
                .unwrap_or_else(|| format!("Service {}", service.service_id));

            result.push(MirakurunChannel {
                name,
                channel_type: channel_type.to_string(),
                channel: channel_value.clone(),
                space,
                service_id: Some(service.service_id as u16),
            });
        }
    }

    sort_channels(&mut result);
    result
}

/// Generate YAML string for Mirakurun channels
pub fn to_yaml(channels: &[MirakurunChannel]) -> Result<String> {
    // Use serde_yaml-like manual formatting for clean output
    let mut output = String::new();

    for channel in channels {
        output.push_str(&format!("- name: {}\n", escape_yaml_string(&channel.name)));
        output.push_str(&format!("  type: {}\n", channel.channel_type));
        output.push_str(&format!("  channel: '{}'\n", channel.channel));
        if let Some(space) = channel.space {
            output.push_str(&format!("  space: {}\n", space));
        }
        if let Some(service_id) = channel.service_id {
            output.push_str(&format!("  serviceId: {}\n", service_id));
        }
        output.push('\n');
    }

    Ok(output)
}

/// Escape special characters in YAML string values
fn escape_yaml_string(s: &str) -> String {
    // If the string contains special characters, quote it
    if s.contains(':')
        || s.contains('#')
        || s.contains('\'')
        || s.contains('"')
        || s.contains('\n')
        || s.starts_with(' ')
        || s.ends_with(' ')
    {
        // Use double quotes and escape internal double quotes
        format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        s.to_string()
    }
}

/// Write Mirakurun channels to stdout
pub fn write_mirakurun_stdout(channels: &[ScannedChannelInfo]) -> Result<()> {
    let mirakurun_channels = to_mirakurun_channels(channels);
    let yaml = to_yaml(&mirakurun_channels)?;
    print!("{}", yaml);
    Ok(())
}

/// Write Mirakurun channels to file
pub fn write_mirakurun_file(channels: &[ScannedChannelInfo], path: &std::path::Path) -> Result<()> {
    let mirakurun_channels = to_mirakurun_channels(channels);
    let yaml = to_yaml(&mirakurun_channels)?;
    let mut file = std::fs::File::create(path)?;
    file.write_all(yaml.as_bytes())?;
    Ok(())
}

/// Write exported Mirakurun channels to stdout
pub fn write_export_mirakurun_stdout(channels: &[ExportedChannel]) -> Result<()> {
    let mirakurun_channels = exported_to_mirakurun_channels(channels);
    let yaml = to_yaml(&mirakurun_channels)?;
    print!("{}", yaml);
    Ok(())
}

/// Write exported Mirakurun channels to file
pub fn write_export_mirakurun_file(
    channels: &[ExportedChannel],
    path: &std::path::Path,
) -> Result<()> {
    let mirakurun_channels = exported_to_mirakurun_channels(channels);
    let yaml = to_yaml(&mirakurun_channels)?;
    let mut file = std::fs::File::create(path)?;
    file.write_all(yaml.as_bytes())?;
    Ok(())
}
