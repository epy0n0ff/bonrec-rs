//! Channel scanning module

pub mod channel;
pub mod si_parser;

pub use channel::{ChannelScanner, ScanConfig, ScannedChannelInfo, DEFAULT_SI_TIMEOUT_SECS};
pub use si_parser::{SiParser, ServiceInfo};
