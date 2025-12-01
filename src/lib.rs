//! bonrec-rs - BonDriver Channel Scanner
//!
//! A Rust library for scanning TV channels using BonDriver interface
//! on Windows systems.

pub mod bondriver;
pub mod cli;
pub mod error;
pub mod output;
pub mod scanner;
pub mod storage;

pub use error::BonrecError;
