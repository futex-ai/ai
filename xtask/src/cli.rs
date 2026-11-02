//! Command-line parsing for workspace automation.

use ai_testkit::provider::Provider;
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
    /// Regenerate the committed model capability matrix from provider specs.
    CapabilityMatrix,
    /// Audit Rust source files for the repository line-count cap.
    RustFileLengthLint {
        /// Audit every Rust file under crates/ and xtask/.
        #[arg(long)]
        all: bool,
    },
    /// Run a credential-free runtime construction smoke test.
    SmokeTest,
    /// Record provider success scenarios with RECORD_API_KEY.
    RecordStreams {
        /// Provider name: anthropic, openai, google, deepseek, kimi, qwen, minimax, xai.
        #[arg(long)]
        provider: Provider,
    },
}

#[cfg(test)]
#[path = "_tests_/record_streams_cli_tests.rs"]
mod record_streams_cli_tests;
