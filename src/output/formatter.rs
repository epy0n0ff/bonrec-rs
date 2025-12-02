//! Output formatter implementations

use std::io::Write;
use std::path::Path;

use crate::error::Result;
use crate::output::{ExportResult, ScanResult, ScannedChannel};

/// Write scan result to a file as JSON
pub fn write_json_file(result: &ScanResult, path: &Path) -> Result<()> {
    let json = result.to_json()?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Write scan result to stdout as CSV
pub fn write_csv_stdout(result: &ScanResult) -> Result<()> {
    let mut writer = std::io::stdout();
    write_csv(&mut writer, result)
}

/// Write scan result to a file as CSV
pub fn write_csv_file(result: &ScanResult, path: &Path) -> Result<()> {
    let file = std::fs::File::create(path)?;
    let mut writer = std::io::BufWriter::new(file);
    write_csv(&mut writer, result)
}

/// Write scan result as CSV to any writer
fn write_csv<W: Write>(writer: &mut W, result: &ScanResult) -> Result<()> {
    write_channels_csv(writer, &result.channels)
}

/// Write export result to stdout as CSV
pub fn write_export_csv_stdout(result: &ExportResult) -> Result<()> {
    let mut writer = std::io::stdout();
    write_channels_csv(&mut writer, &result.channels)
}

/// Write export result to a file as CSV
pub fn write_export_csv_file(result: &ExportResult, path: &Path) -> Result<()> {
    let file = std::fs::File::create(path)?;
    let mut writer = std::io::BufWriter::new(file);
    write_channels_csv(&mut writer, &result.channels)
}

/// Write channels as CSV to any writer
fn write_channels_csv<W: Write>(writer: &mut W, channels: &[ScannedChannel]) -> Result<()> {
    // Write header
    writeln!(
        writer,
        "tuning_space,channel_index,channel_name,physical_channel,service_id,network_id,transport_stream_id,service_name,broadcaster_name,available_from"
    )?;

    // Write data rows
    for channel in channels {
        let available_from = channel.available_from.join(";");

        if channel.services.is_empty() {
            // Channel with no services
            writeln!(
                writer,
                "{},{},{},{},,,,,{}",
                escape_csv(&channel.tuning_space),
                channel.channel_index,
                channel
                    .channel_name
                    .as_ref()
                    .map(|s| escape_csv(s))
                    .unwrap_or_default(),
                channel
                    .physical_channel
                    .map(|n| n.to_string())
                    .unwrap_or_default(),
                escape_csv(&available_from),
            )?;
        } else {
            // One row per service
            for service in &channel.services {
                writeln!(
                    writer,
                    "{},{},{},{},{},{},{},{},{},{}",
                    escape_csv(&channel.tuning_space),
                    channel.channel_index,
                    channel
                        .channel_name
                        .as_ref()
                        .map(|s| escape_csv(s))
                        .unwrap_or_default(),
                    channel
                        .physical_channel
                        .map(|n| n.to_string())
                        .unwrap_or_default(),
                    service.service_id,
                    service
                        .network_id
                        .map(|n| n.to_string())
                        .unwrap_or_default(),
                    service
                        .transport_stream_id
                        .map(|n| n.to_string())
                        .unwrap_or_default(),
                    service
                        .service_name
                        .as_ref()
                        .map(|s| escape_csv(s))
                        .unwrap_or_default(),
                    service
                        .broadcaster_name
                        .as_ref()
                        .map(|s| escape_csv(s))
                        .unwrap_or_default(),
                    escape_csv(&available_from),
                )?;
            }
        }
    }

    writer.flush()?;
    Ok(())
}

/// Escape a string for CSV output
fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_csv_simple() {
        assert_eq!(escape_csv("hello"), "hello");
    }

    #[test]
    fn test_escape_csv_with_comma() {
        assert_eq!(escape_csv("hello,world"), "\"hello,world\"");
    }

    #[test]
    fn test_escape_csv_with_quotes() {
        assert_eq!(escape_csv("say \"hello\""), "\"say \"\"hello\"\"\"");
    }

    #[test]
    fn test_escape_csv_with_newline() {
        assert_eq!(escape_csv("hello\nworld"), "\"hello\nworld\"");
    }
}
