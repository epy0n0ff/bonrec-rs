//! CLI module

pub mod args;

use std::process::ExitCode;

use clap::Parser;

use crate::bondriver::Tuner;
use crate::error::BonrecError;
use crate::output::{write_csv_file, write_csv_stdout, write_json_file, write_json_stdout, ScanResult};
use crate::scanner::{ChannelScanner, ScanConfig};
use crate::storage::{BonDriverSource, Database, ScanSession, ScanStatus};

pub use args::{Cli, Commands, OutputFormat, ScanArgs};

/// Run the CLI application
pub fn run() -> ExitCode {
    let cli = Cli::parse();

    // Set log level based on verbose flag
    if cli.verbose {
        std::env::set_var("RUST_LOG", "debug");
    } else {
        std::env::set_var("RUST_LOG", "info");
    }

    match execute_command(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error: {}", e);
            ExitCode::from(e.exit_code() as u8)
        }
    }
}

/// Execute the CLI command
fn execute_command(cli: Cli) -> crate::error::Result<()> {
    match cli.command {
        Commands::Scan(args) => execute_scan(args),
    }
}

/// Execute the scan command
fn execute_scan(args: ScanArgs) -> crate::error::Result<()> {
    // Open database
    let db = Database::open(&args.db)?;
    log::info!("Database opened: {}", args.db.display());

    // Create scan session
    let mut session = ScanSession::new_running();
    db.create_scan_session(&mut session)?;
    log::info!("Started scan session #{}", session.id);

    // Create scanner config
    let config = ScanConfig {
        si_timeout_secs: args.timeout,
        space_filter: args.space.clone(),
        show_progress: !args.no_progress,
    };

    let mut scanner = ChannelScanner::new(config);

    // Set up Ctrl+C handler
    scanner.setup_ctrlc_handler()?;

    // Scan each BonDriver
    for bondriver_path in &args.bondriver {
        if scanner.is_interrupted() {
            break;
        }

        log::info!("Loading BonDriver: {}", bondriver_path.display());

        // Load and open tuner
        let mut tuner = match Tuner::new(bondriver_path) {
            Ok(t) => t,
            Err(e) => {
                log::error!("Failed to load BonDriver: {}", e);
                continue;
            }
        };

        if let Err(e) = tuner.open() {
            log::error!("Failed to open tuner: {}", e);
            continue;
        }

        // Get tuner name
        let tuner_name = tuner.get_tuner_name();
        log::info!("Tuner: {}", tuner_name.as_deref().unwrap_or("Unknown"));

        // Create BonDriver source record
        let mut bondriver_source =
            BonDriverSource::new(session.id, bondriver_path.to_string_lossy());
        bondriver_source.tuner_name = tuner_name;
        db.create_bondriver_source(&mut bondriver_source)?;

        // Scan channels
        match scanner.scan_bondriver(&mut tuner, &db, &session, &mut bondriver_source) {
            Ok(results) => {
                log::info!(
                    "Scanned {} channels from {}",
                    results.len(),
                    bondriver_path.display()
                );
            }
            Err(BonrecError::Interrupted) => {
                log::info!("Scan interrupted by user");
                break;
            }
            Err(e) => {
                log::error!("Scan error: {}", e);
            }
        }
    }

    // Update session status
    if scanner.is_interrupted() {
        session.interrupt();
    } else {
        session.complete();
    }
    db.update_scan_session(&session)?;

    // Get merged results
    let channels = scanner.get_merged_channels();
    log::info!("Total unique channels: {}", channels.len());

    // Create output result
    let result = ScanResult::new(
        session.id,
        session.started_at,
        session.completed_at,
        session.status,
        channels,
    );

    // Output result based on format and destination
    match args.format {
        OutputFormat::Json => {
            if let Some(output_path) = args.output {
                // Write to file
                write_json_file(&result, &output_path)?;
                log::info!("Output written to: {}", output_path.display());
            } else {
                // Write to stdout
                write_json_stdout(&result)?;
            }
        }
        OutputFormat::Csv => {
            if let Some(output_path) = args.output {
                // Write to file
                write_csv_file(&result, &output_path)?;
                log::info!("Output written to: {}", output_path.display());
            } else {
                // Write to stdout
                write_csv_stdout(&result)?;
            }
        }
    }

    // Return error if interrupted
    if session.status == ScanStatus::Interrupted {
        return Err(BonrecError::Interrupted);
    }

    Ok(())
}
