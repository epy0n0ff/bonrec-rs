//! JSON output formatting

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::scanner::{ScannedChannelInfo, ServiceInfo};
use crate::storage::ScanStatus;

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
