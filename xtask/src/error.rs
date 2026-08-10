//! Error contract for workspace automation.

use std::io;
use std::path::PathBuf;
use std::process::ExitStatus;

use thiserror::Error;

pub(crate) type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub(crate) enum Error {
    #[error("[xtask/workspace] CARGO_MANIFEST_DIR is not set")]
    MissingManifestDir,
    #[error("[xtask/workspace] manifest directory has no workspace parent: {manifest_dir:?}")]
    MissingWorkspaceParent { manifest_dir: PathBuf },
    #[error("[xtask/check] failed to start `{command}`: {source}")]
    CommandStart { command: String, source: io::Error },
    #[error("[xtask/check] `{command}` failed with status {status}")]
    CommandFailed { command: String, status: ExitStatus },
    #[error("[xtask/rust_file_length_lint] failed to read directory `{path}`: {source}")]
    ReadDir { path: PathBuf, source: io::Error },
    #[error("[xtask/rust_file_length_lint] failed to read Rust file `{path}`: {source}")]
    ReadFile { path: PathBuf, source: io::Error },
    #[error("[xtask/rust_file_length_lint] found {count} violation(s):\n{details}")]
    FileLengthViolations { count: usize, details: String },
    #[error("[xtask/smoke] failed to build tool-calling runtime: {source}")]
    SmokeRuntime { source: ai_tool_calling::Error },
    #[error("[xtask/smoke] failed to build MCP tool adapter: {source}")]
    SmokeMcp { source: ai_mcp::Error },
    #[error("[xtask/smoke] failed to build MCP OAuth boundary: {source}")]
    SmokeMcpOAuth { source: ai_mcp_oauth::Error },
}
