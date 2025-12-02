//! CLI argument definitions

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::scanner::DEFAULT_SI_TIMEOUT_SECS;

/// BonDriver channel scanner
#[derive(Parser, Debug)]
#[command(name = "bonrec-rs")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Subcommand to execute
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose logging (debug level)
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Enable trace logging (most detailed, for debugging TS stream issues)
    #[arg(long, global = true)]
    pub trace: bool,
}

/// Available subcommands
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Scan channels using BonDriver
    Scan(ScanArgs),
    /// Export scan results from database
    Export(ExportArgs),
}

/// Arguments for the scan command
#[derive(Parser, Debug)]
pub struct ScanArgs {
    /// BonDriver DLL file paths (can specify multiple)
    #[arg(required = true)]
    pub bondriver: Vec<PathBuf>,

    /// Output file path (default: stdout)
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Output format (json or csv)
    #[arg(short, long, default_value = "json")]
    pub format: OutputFormat,

    /// SI information timeout in seconds
    #[arg(short, long, default_value_t = DEFAULT_SI_TIMEOUT_SECS)]
    pub timeout: u64,

    /// Tuning spaces to scan (can specify multiple, e.g., -s "地上D" -s "BS")
    #[arg(short, long)]
    pub space: Vec<String>,

    /// SQLite database path
    #[arg(long, default_value = "bonrec.db")]
    pub db: PathBuf,

    /// Disable progress bar
    #[arg(long)]
    pub no_progress: bool,
}

/// Output format
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputFormat {
    /// JSON format
    Json,
    /// CSV format
    Csv,
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Json => write!(f, "json"),
            OutputFormat::Csv => write!(f, "csv"),
        }
    }
}

/// Arguments for the export command
#[derive(Parser, Debug)]
pub struct ExportArgs {
    /// Scan session ID to export
    #[arg(long, group = "session_select")]
    pub session: Option<i64>,

    /// Export the latest scan session
    #[arg(long, group = "session_select")]
    pub latest: bool,

    /// Output file path (default: stdout)
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Output format (json or csv)
    #[arg(short, long, default_value = "json")]
    pub format: OutputFormat,

    /// SQLite database path
    #[arg(long, default_value = "bonrec.db")]
    pub db: PathBuf,
}
