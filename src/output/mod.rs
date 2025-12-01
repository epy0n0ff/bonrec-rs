//! Output formatting module

pub mod formatter;
pub mod json;

pub use formatter::{write_csv_file, write_csv_stdout, write_json_file};
pub use json::{write_json_stdout, ScanResult, ScannedChannel, ScannedService};
