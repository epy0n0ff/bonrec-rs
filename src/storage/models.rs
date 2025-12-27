//! Data model structs for storage layer

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Scan session status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanStatus {
    /// Scan is currently running
    Running,
    /// Scan completed successfully
    Completed,
    /// Scan was interrupted by user
    Interrupted,
}

impl ScanStatus {
    /// Convert to string for database storage
    pub fn as_str(&self) -> &'static str {
        match self {
            ScanStatus::Running => "running",
            ScanStatus::Completed => "completed",
            ScanStatus::Interrupted => "interrupted",
        }
    }

    /// Parse from database string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "running" => Some(ScanStatus::Running),
            "completed" => Some(ScanStatus::Completed),
            "interrupted" => Some(ScanStatus::Interrupted),
            _ => None,
        }
    }
}

impl std::fmt::Display for ScanStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Scan session record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanSession {
    /// Primary key
    pub id: i64,
    /// Scan start time
    pub started_at: DateTime<Utc>,
    /// Scan completion time (None if interrupted or still running)
    pub completed_at: Option<DateTime<Utc>>,
    /// Current status
    pub status: ScanStatus,
}

impl ScanSession {
    /// Create a new running scan session
    pub fn new_running() -> Self {
        ScanSession {
            id: 0,
            started_at: Utc::now(),
            completed_at: None,
            status: ScanStatus::Running,
        }
    }

    /// Mark session as completed
    pub fn complete(&mut self) {
        self.completed_at = Some(Utc::now());
        self.status = ScanStatus::Completed;
    }

    /// Mark session as interrupted
    pub fn interrupt(&mut self) {
        self.completed_at = Some(Utc::now());
        self.status = ScanStatus::Interrupted;
    }
}

/// BonDriver source record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BonDriverSource {
    /// Primary key
    pub id: i64,
    /// Reference to scan session
    pub scan_session_id: i64,
    /// Path to the DLL file
    pub dll_path: String,
    /// Tuner name from GetTunerName
    pub tuner_name: Option<String>,
}

impl BonDriverSource {
    /// Create a new BonDriver source
    pub fn new(scan_session_id: i64, dll_path: impl Into<String>) -> Self {
        BonDriverSource {
            id: 0,
            scan_session_id,
            dll_path: dll_path.into(),
            tuner_name: None,
        }
    }
}

/// Tuning space record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuningSpace {
    /// Primary key
    pub id: i64,
    /// Reference to BonDriver source
    pub bondriver_source_id: i64,
    /// Space index in BonDriver
    pub space_index: u32,
    /// Space name (e.g., "地上D", "BS", "CS110")
    pub space_name: String,
}

impl TuningSpace {
    /// Create a new tuning space
    pub fn new(
        bondriver_source_id: i64,
        space_index: u32,
        space_name: impl Into<String>,
    ) -> Self {
        TuningSpace {
            id: 0,
            bondriver_source_id,
            space_index,
            space_name: space_name.into(),
        }
    }
}

/// Channel record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    /// Primary key
    pub id: i64,
    /// Reference to tuning space
    pub tuning_space_id: i64,
    /// Channel index in BonDriver
    pub channel_index: u32,
    /// Channel name from EnumChannelName
    pub channel_name: Option<String>,
    /// Physical channel number
    pub physical_channel: Option<u32>,
}

impl Channel {
    /// Create a new channel
    pub fn new(tuning_space_id: i64, channel_index: u32) -> Self {
        Channel {
            id: 0,
            tuning_space_id,
            channel_index,
            channel_name: None,
            physical_channel: None,
        }
    }
}

/// Service record (one channel can have multiple services)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Service {
    /// Primary key
    pub id: i64,
    /// Reference to channel
    pub channel_id: i64,
    /// Service ID (MPEG-2 program_number)
    pub service_id: u16,
    /// Network ID from NIT
    pub network_id: Option<u16>,
    /// Transport stream ID
    pub transport_stream_id: Option<u16>,
    /// Service name from SDT
    pub service_name: Option<String>,
    /// Broadcaster name from SDT
    pub broadcaster_name: Option<String>,
}

impl Service {
    /// Create a new service
    pub fn new(channel_id: i64, service_id: u16) -> Self {
        Service {
            id: 0,
            channel_id,
            service_id,
            network_id: None,
            transport_stream_id: None,
            service_name: None,
            broadcaster_name: None,
        }
    }
}

/// Channel source relation (many-to-many: channel <-> BonDriver)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelSource {
    /// Reference to channel
    pub channel_id: i64,
    /// Reference to BonDriver source
    pub bondriver_source_id: i64,
}

impl ChannelSource {
    /// Create a new channel source relation
    pub fn new(channel_id: i64, bondriver_source_id: i64) -> Self {
        ChannelSource {
            channel_id,
            bondriver_source_id,
        }
    }
}
