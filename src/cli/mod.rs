//! CLI module

pub mod args;

use std::process::ExitCode;

use clap::Parser;

use crate::bondriver::Tuner;
use crate::error::BonrecError;
use crate::output::{
    write_csv_file, write_csv_stdout, write_json_file, write_json_stdout,
    write_mirakurun_file, write_mirakurun_stdout, ScanResult,
};
use crate::scanner::{ChannelScanner, ScanConfig};
use crate::storage::{BonDriverSource, Database, ScanSession, ScanStatus};

pub use args::{Cli, Commands, ExportArgs, OutputFormat, ScanArgs};

/// Run the CLI application
pub fn run() -> ExitCode {
    let cli = Cli::parse();

    // Set log level based on verbose/trace flags and initialize logger
    if cli.trace {
        std::env::set_var("RUST_LOG", "trace");
    } else if cli.verbose {
        std::env::set_var("RUST_LOG", "debug");
    } else {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

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
        Commands::Export(args) => execute_export(args),
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

    // Output result based on format and destination
    match args.format {
        OutputFormat::Json => {
            let result = ScanResult::new(
                session.id,
                session.started_at,
                session.completed_at,
                session.status,
                channels,
            );
            if let Some(output_path) = args.output {
                write_json_file(&result, &output_path)?;
                log::info!("Output written to: {}", output_path.display());
            } else {
                write_json_stdout(&result)?;
            }
        }
        OutputFormat::Csv => {
            let result = ScanResult::new(
                session.id,
                session.started_at,
                session.completed_at,
                session.status,
                channels,
            );
            if let Some(output_path) = args.output {
                write_csv_file(&result, &output_path)?;
                log::info!("Output written to: {}", output_path.display());
            } else {
                write_csv_stdout(&result)?;
            }
        }
        OutputFormat::Mirakurun => {
            if let Some(output_path) = args.output {
                write_mirakurun_file(&channels, &output_path)?;
                log::info!("Output written to: {}", output_path.display());
            } else {
                write_mirakurun_stdout(&channels)?;
            }
        }
    }

    // Return error if interrupted
    if session.status == ScanStatus::Interrupted {
        return Err(BonrecError::Interrupted);
    }

    Ok(())
}

/// Execute the export command
fn execute_export(args: ExportArgs) -> crate::error::Result<()> {
    use crate::output::{
        write_export_csv_file, write_export_csv_stdout, write_export_json_file,
        write_export_json_stdout, write_export_mirakurun_file, write_export_mirakurun_stdout,
        ExportResult,
    };

    // Open database
    let db = Database::open(&args.db)?;
    log::info!("Database opened: {}", args.db.display());

    // Get the session to export
    let session = if args.latest {
        db.get_latest_scan_session()?
            .ok_or_else(|| BonrecError::invalid_argument("No scan sessions found in database"))?
    } else if let Some(session_id) = args.session {
        db.get_scan_session(session_id)?
            .ok_or_else(|| BonrecError::invalid_argument(format!("Scan session {} not found", session_id)))?
    } else {
        // Default to latest if neither specified
        db.get_latest_scan_session()?
            .ok_or_else(|| BonrecError::invalid_argument("No scan sessions found in database"))?
    };

    log::info!("Exporting session #{} ({})", session.id, session.status.as_str());

    // Get channels for the session
    let channels = db.get_session_channels(session.id)?;
    log::info!("Found {} channels", channels.len());

    // Output result based on format and destination
    match args.format {
        OutputFormat::Json => {
            let result = ExportResult::new(
                session.id,
                session.started_at,
                session.completed_at,
                session.status,
                channels,
            );
            if let Some(output_path) = args.output {
                write_export_json_file(&result, &output_path)?;
                log::info!("Output written to: {}", output_path.display());
            } else {
                write_export_json_stdout(&result)?;
            }
        }
        OutputFormat::Csv => {
            let result = ExportResult::new(
                session.id,
                session.started_at,
                session.completed_at,
                session.status,
                channels,
            );
            if let Some(output_path) = args.output {
                write_export_csv_file(&result, &output_path)?;
                log::info!("Output written to: {}", output_path.display());
            } else {
                write_export_csv_stdout(&result)?;
            }
        }
        OutputFormat::Mirakurun => {
            if let Some(output_path) = args.output {
                write_export_mirakurun_file(&channels, &output_path)?;
                log::info!("Output written to: {}", output_path.display());
            } else {
                write_export_mirakurun_stdout(&channels)?;
            }
        }
    }

    Ok(())
}
