//! JSON output formatting

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::scanner::{ScannedChannelInfo, ServiceInfo};
use crate::storage::{ExportedChannel, ScanStatus, Service};

/// Scanned service for output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannedService {
    /// Service ID
    pub service_id: u16,
    /// Network ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network_id: Option<u16>,
    /// Transport stream ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transport_stream_id: Option<u16>,
    /// Service name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_name: Option<String>,
    /// Broadcaster name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub broadcaster_name: Option<String>,
}

impl From<&ServiceInfo> for ScannedService {
    fn from(info: &ServiceInfo) -> Self {
        ScannedService {
            service_id: info.service_id,
            network_id: info.network_id,
            transport_stream_id: info.transport_stream_id,
            service_name: info.service_name.clone(),
            broadcaster_name: info.provider_name.clone(),
        }
    }
}

/// Scanned channel for output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannedChannel {
    /// Tuning space name
    pub tuning_space: String,
    /// Channel index
    pub channel_index: u32,
    /// Channel name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_name: Option<String>,
    /// Physical channel number
    #[serde(skip_serializing_if = "Option::is_none")]
    pub physical_channel: Option<u32>,
    /// Services on this channel
    pub services: Vec<ScannedService>,
    /// BonDriver paths that can access this channel
    pub available_from: Vec<String>,
}

impl From<&ScannedChannelInfo> for ScannedChannel {
    fn from(info: &ScannedChannelInfo) -> Self {
        ScannedChannel {
            tuning_space: info.tuning_space.clone(),
            channel_index: info.channel_index,
            channel_name: info.channel_name.clone(),
            physical_channel: info.physical_channel,
            services: info.services.iter().map(ScannedService::from).collect(),
            available_from: info.available_from.clone(),
        }
    }
}

/// Complete scan result for output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    /// Scan session ID
    pub scan_session_id: i64,
    /// Scan start time
    pub started_at: DateTime<Utc>,
    /// Scan completion time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    /// Scan status
    pub status: ScanStatus,
    /// Scanned channels
    pub channels: Vec<ScannedChannel>,
}

impl ScanResult {
    /// Create a new scan result
    pub fn new(
        scan_session_id: i64,
        started_at: DateTime<Utc>,
        completed_at: Option<DateTime<Utc>>,
        status: ScanStatus,
        channels: Vec<ScannedChannelInfo>,
    ) -> Self {
        ScanResult {
            scan_session_id,
            started_at,
            completed_at,
            status,
            channels: channels.iter().map(ScannedChannel::from).collect(),
        }
    }

    /// Convert to JSON string
    pub fn to_json(&self) -> crate::error::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Convert to compact JSON string (no pretty printing)
    pub fn to_json_compact(&self) -> crate::error::Result<String> {
        Ok(serde_json::to_string(self)?)
    }
}

/// Write scan result to stdout as JSON
pub fn write_json_stdout(result: &ScanResult) -> crate::error::Result<()> {
    let json = result.to_json()?;
    println!("{}", json);
    Ok(())
}

// ========== Export Result (from database) ==========

impl From<&Service> for ScannedService {
    fn from(svc: &Service) -> Self {
        ScannedService {
            service_id: svc.service_id as u16,
            network_id: svc.network_id.map(|n| n as u16),
            transport_stream_id: svc.transport_stream_id.map(|n| n as u16),
            service_name: svc.service_name.clone(),
            broadcaster_name: svc.broadcaster_name.clone(),
        }
    }
}

impl From<&ExportedChannel> for ScannedChannel {
    fn from(ch: &ExportedChannel) -> Self {
        ScannedChannel {
            tuning_space: ch.tuning_space.clone(),
            channel_index: ch.channel_index,
            channel_name: ch.channel_name.clone(),
            physical_channel: ch.physical_channel,
            services: ch.services.iter().map(ScannedService::from).collect(),
            available_from: ch.available_from.clone(),
        }
    }
}

/// Export result from database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportResult {
    /// Scan session ID
    pub scan_session_id: i64,
    /// Scan start time
    pub started_at: DateTime<Utc>,
    /// Scan completion time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    /// Scan status
    pub status: ScanStatus,
    /// Scanned channels
    pub channels: Vec<ScannedChannel>,
}

impl ExportResult {
    /// Create a new export result from database data
    pub fn new(
        scan_session_id: i64,
        started_at: DateTime<Utc>,
        completed_at: Option<DateTime<Utc>>,
        status: ScanStatus,
        channels: Vec<ExportedChannel>,
    ) -> Self {
        ExportResult {
            scan_session_id,
            started_at,
            completed_at,
            status,
            channels: channels.iter().map(ScannedChannel::from).collect(),
        }
    }

    /// Convert to JSON string
    pub fn to_json(&self) -> crate::error::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

/// Write export result to stdout as JSON
pub fn write_export_json_stdout(result: &ExportResult) -> crate::error::Result<()> {
    let json = result.to_json()?;
    println!("{}", json);
    Ok(())
}

/// Write export result to file as JSON
pub fn write_export_json_file(result: &ExportResult, path: &std::path::Path) -> crate::error::Result<()> {
    let json = result.to_json()?;
    std::fs::write(path, json)?;
    Ok(())
}
