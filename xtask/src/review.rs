//! Codex review command delegation.

use std::path::Path;

use crate::command;
use crate::error::Result;

pub(crate) fn run(root: &Path) -> Result<()> {
    command::run(
        root,
        "codex",
        &[
            "exec",
            "review",
            "--base",
            "origin/main",
            "--ephemeral",
            "--skip-git-repo-check",
        ],
    )
}
