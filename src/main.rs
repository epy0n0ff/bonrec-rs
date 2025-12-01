//! bonrec-rs - BonDriver Channel Scanner CLI
//!
//! Entry point for the channel scanning command-line tool.

use bonrec::cli;
use std::process::ExitCode;

fn main() -> ExitCode {
    env_logger::init();
    cli::run()
}
