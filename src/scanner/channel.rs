//! Channel scanning logic

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use indicatif::{ProgressBar, ProgressStyle};

use crate::bondriver::Tuner;
use crate::error::{BonrecError, Result};
use crate::storage::{
    BonDriverSource, Channel, ChannelSource, Database, ScanSession, Service, TuningSpace,
};

use super::si_parser::{ServiceInfo, SiParser};

/// Default SI timeout in seconds
pub const DEFAULT_SI_TIMEOUT_SECS: u64 = 5;

/// Scan configuration
#[derive(Debug, Clone)]
pub struct ScanConfig {
    /// SI information timeout in seconds
    pub si_timeout_secs: u64,
    /// Filter by specific tuning space names (empty = all)
    pub space_filter: Vec<String>,
    /// Show progress bar
    pub show_progress: bool,
}

impl Default for ScanConfig {
    fn default() -> Self {
        ScanConfig {
            si_timeout_secs: DEFAULT_SI_TIMEOUT_SECS,
            space_filter: Vec::new(),
            show_progress: true,
        }
    }
}

/// Scanned channel result
#[derive(Debug, Clone)]
pub struct ScannedChannelInfo {
    /// Tuning space name
    pub tuning_space: String,
    /// Channel index
    pub channel_index: u32,
    /// Channel name from BonDriver
    pub channel_name: Option<String>,
    /// Physical channel number
    pub physical_channel: Option<u32>,
    /// Signal level
    pub signal_level: f32,
    /// Services found on this channel
    pub services: Vec<ServiceInfo>,
    /// BonDriver paths that can access this channel
    pub available_from: Vec<String>,
}

/// Channel scanner
pub struct ChannelScanner {
    /// Scan configuration
    config: ScanConfig,
    /// Interrupt flag
    interrupted: Arc<AtomicBool>,
    /// Scanned channels (key: tuning_space:channel_index)
    channels: HashMap<String, ScannedChannelInfo>,
}

impl ChannelScanner {
    /// Create a new channel scanner
    pub fn new(config: ScanConfig) -> Self {
        ChannelScanner {
            config,
            interrupted: Arc::new(AtomicBool::new(false)),
            channels: HashMap::new(),
        }
    }

    /// Get interrupt flag for external use
    pub fn interrupt_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.interrupted)
    }

    /// Check if scan was interrupted
    pub fn is_interrupted(&self) -> bool {
        self.interrupted.load(Ordering::SeqCst)
    }

    /// Set up Ctrl+C handler
    pub fn setup_ctrlc_handler(&self) -> Result<()> {
        let flag = Arc::clone(&self.interrupted);
        ctrlc::set_handler(move || {
            log::info!("Received interrupt signal, stopping scan...");
            flag.store(true, Ordering::SeqCst);
        })
        .map_err(|e| BonrecError::operation_error(format!("Failed to set Ctrl+C handler: {}", e)))
    }

    /// Scan a single channel and get SI information
    pub fn scan_channel(
        &self,
        tuner: &mut Tuner,
        space: u32,
        channel: u32,
    ) -> Result<Option<Vec<ServiceInfo>>> {
        // Set channel
        tuner.set_channel(space, channel)?;

        // Purge any stale data
        tuner.purge_ts_stream();

        // Wait a bit for tuning to stabilize
        std::thread::sleep(Duration::from_millis(500));

        // Check signal level (some BonDrivers may return 0 even with signal)
        let signal_level = tuner.get_signal_level();
        log::debug!(
            "Scanning space={}, channel={}, signal={:.2}dB",
            space,
            channel,
            signal_level
        );

        // Only skip if signal level is clearly indicating no signal
        // Some BonDrivers return 0.0 or negative values even with valid signal
        if signal_level < -100.0 {
            log::debug!(
                "Very weak signal on space={}, channel={}: {:.2}dB, skipping",
                space,
                channel,
                signal_level
            );
            return Ok(None);
        }

        // Collect SI information with timeout
        let mut parser = SiParser::new();
        let start_time = Instant::now();
        let timeout = Duration::from_secs(self.config.si_timeout_secs);

        let mut total_bytes_read = 0u64;
        let mut read_attempts = 0u32;
        let mut buffer = vec![0u8; 188 * 256]; // ~48KB buffer

        while start_time.elapsed() < timeout {
            if self.is_interrupted() {
                return Err(BonrecError::Interrupted);
            }

            read_attempts += 1;

            // Call WaitTsStream to trigger internal data preparation (even if return value is 0)
            let wait_result = tuner.wait_ts_stream(100);
            log::trace!("WaitTsStream returned: {}", wait_result);

            // Try to read TS data
            let mut all_data = Vec::new();

            loop {
                match tuner.get_ts_stream(&mut buffer) {
                    Ok((bytes_read, remain)) => {
                        if bytes_read == 0 {
                            break;
                        }
                        all_data.extend_from_slice(&buffer[..bytes_read]);
                        log::trace!("Read {} bytes, {} remaining", bytes_read, remain);
                        if remain == 0 {
                            break;
                        }
                    }
                    Err(_) => {
                        // No data available or error, wait a bit and retry
                        break;
                    }
                }
            }

            if all_data.is_empty() {
                // No data available, wait a bit before retrying
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }

            total_bytes_read += all_data.len() as u64;
            log::debug!(
                "Read {} bytes of TS data (total: {} bytes, attempt {})",
                all_data.len(),
                total_bytes_read,
                read_attempts
            );

            // Parse SI information
            parser.parse(&all_data)?;

            // Check if we have enough information
            if parser.has_basic_info() && parser.has_service_names() {
                log::debug!(
                    "SI info complete for space={}, channel={}",
                    space,
                    channel
                );
                break;
            }

            // Small delay between read attempts
            std::thread::sleep(Duration::from_millis(50));
        }

        log::debug!(
            "Scan complete: {} attempts, {} bytes read, PAT={}, SDT={}",
            read_attempts,
            total_bytes_read,
            parser.has_basic_info(),
            parser.has_service_names()
        );

        if !parser.has_basic_info() {
            log::debug!(
                "No PAT found on space={}, channel={} (timeout after {}s, {} bytes read)",
                space,
                channel,
                self.config.si_timeout_secs,
                total_bytes_read
            );
            return Ok(None);
        }

        if !parser.has_service_names() {
            log::debug!(
                "PAT found but no SDT on space={}, channel={}, continuing with partial info",
                space,
                channel
            );
        }

        let services = parser.get_services();
        log::info!(
            "Found {} services on space={}, channel={}",
            services.len(),
            space,
            channel
        );

        Ok(Some(services))
    }

    /// Scan all channels in a tuning space
    pub fn scan_tuning_space(
        &mut self,
        tuner: &mut Tuner,
        space_index: u32,
        space_name: &str,
        progress: Option<&ProgressBar>,
    ) -> Result<Vec<ScannedChannelInfo>> {
        let channels = tuner.get_channels(space_index);
        let mut results = Vec::new();

        for (channel_index, channel_name) in channels {
            if self.is_interrupted() {
                log::info!("Scan interrupted at space={}, channel={}", space_index, channel_index);
                break;
            }

            if let Some(pb) = progress {
                pb.set_message(format!("{} ch{}", space_name, channel_index));
            }

            // Scan channel
            match self.scan_channel(tuner, space_index, channel_index) {
                Ok(Some(services)) => {
                    let signal_level = tuner.get_signal_level();
                    let info = ScannedChannelInfo {
                        tuning_space: space_name.to_string(),
                        channel_index,
                        channel_name: Some(channel_name),
                        physical_channel: Some(channel_index),
                        signal_level,
                        services,
                        available_from: vec![tuner.dll_path().to_string()],
                    };
                    results.push(info);
                }
                Ok(None) => {
                    // No signal or no SI info
                    log::debug!("Skipping channel {} (no signal/SI)", channel_index);
                }
                Err(BonrecError::Interrupted) => {
                    return Err(BonrecError::Interrupted);
                }
                Err(e) => {
                    log::warn!("Error scanning channel {}: {}", channel_index, e);
                }
            }

            if let Some(pb) = progress {
                pb.inc(1);
            }
        }

        Ok(results)
    }

    /// Scan a BonDriver
    pub fn scan_bondriver(
        &mut self,
        tuner: &mut Tuner,
        db: &Database,
        _session: &ScanSession,
        bondriver_source: &mut BonDriverSource,
    ) -> Result<Vec<ScannedChannelInfo>> {
        // Get tuning spaces
        let spaces = tuner.get_tuning_spaces();

        // Filter spaces if needed
        let spaces: Vec<_> = if self.config.space_filter.is_empty() {
            spaces
        } else {
            spaces
                .into_iter()
                .filter(|(_, name)| {
                    self.config
                        .space_filter
                        .iter()
                        .any(|f| name.contains(f) || f == name)
                })
                .collect()
        };

        if spaces.is_empty() {
            log::warn!("No matching tuning spaces found");
            return Ok(Vec::new());
        }

        // Count total channels for progress
        let total_channels: usize = spaces
            .iter()
            .map(|(idx, _)| tuner.get_channels(*idx).len())
            .sum();

        // Create progress bar
        let progress = if self.config.show_progress {
            let pb = ProgressBar::new(total_channels as u64);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}")
                    .unwrap()
                    .progress_chars("█▉▊▋▌▍▎▏  "),
            );
            Some(pb)
        } else {
            None
        };

        let mut all_results = Vec::new();

        for (space_index, space_name) in &spaces {
            if self.is_interrupted() {
                break;
            }

            log::info!("Scanning tuning space: {} (index={})", space_name, space_index);

            // Save tuning space to database
            let mut tuning_space = TuningSpace::new(bondriver_source.id, *space_index, space_name);
            db.create_tuning_space(&mut tuning_space)?;

            // Scan channels
            let results =
                self.scan_tuning_space(tuner, *space_index, space_name, progress.as_ref())?;

            // Save results to database
            for result in &results {
                let mut channel = Channel::new(tuning_space.id, result.channel_index);
                channel.channel_name = result.channel_name.clone();
                channel.physical_channel = result.physical_channel;
                db.create_channel(&mut channel)?;

                // Save services
                for svc_info in &result.services {
                    let mut service = Service::new(channel.id, svc_info.service_id);
                    service.network_id = svc_info.network_id;
                    service.transport_stream_id = svc_info.transport_stream_id;
                    service.service_name = svc_info.service_name.clone();
                    service.broadcaster_name = svc_info.provider_name.clone();
                    db.create_service(&mut service)?;
                }

                // Save channel source relation
                let channel_source = ChannelSource::new(channel.id, bondriver_source.id);
                db.create_channel_source(&channel_source)?;

                // Merge with existing channels
                let key = format!("{}:{}", result.tuning_space, result.channel_index);
                if let Some(existing) = self.channels.get_mut(&key) {
                    // Add this BonDriver to available sources
                    if !existing
                        .available_from
                        .contains(&tuner.dll_path().to_string())
                    {
                        existing
                            .available_from
                            .push(tuner.dll_path().to_string());
                    }
                } else {
                    self.channels.insert(key, result.clone());
                }
            }

            all_results.extend(results);
        }

        if let Some(pb) = progress {
            pb.finish_with_message("Done");
        }

        Ok(all_results)
    }

    /// Get all scanned channels (merged from multiple BonDrivers)
    pub fn get_merged_channels(&self) -> Vec<ScannedChannelInfo> {
        self.channels.values().cloned().collect()
    }

    /// Clear scanned channels
    pub fn clear(&mut self) {
        self.channels.clear();
    }
}
