//! Command-line parsing for workspace automation.

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "xtask", about = "Workspace automation tasks")]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Debug, PartialEq, Eq, Subcommand)]
pub(crate) enum Command {
    /// Run the full local verification sequence.
    Check,
    /// Run an AI review against origin/main.
    Review,
    /// Audit Rust source files for the repository line-count cap.
    RustFileLengthLint {
        /// Audit every Rust file under crates/ and xtask/.
        #[arg(long)]
        all: bool,
    },
    /// Run a credential-free runtime construction smoke test.
    SmokeTest,
}
