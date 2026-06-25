//! Full local verification plan.

use std::path::Path;

use crate::command;
use crate::error::Result;
use crate::file_length;
use crate::smoke;

pub(crate) fn run(root: &Path) -> Result<()> {
    for command in commands() {
        command::run(root, command.program, command.args)?;
    }

    file_length::run(root, true)?;
    smoke::run()
}

pub(crate) struct CheckCommand {
    pub(crate) program: &'static str,
    pub(crate) args: &'static [&'static str],
}

pub(crate) fn commands() -> Vec<CheckCommand> {
    vec![
        CheckCommand {
            program: "cargo",
            args: &["fmt", "--all", "--", "--check"],
        },
        CheckCommand {
            program: "cargo",
            args: &["clippy", "--workspace", "--all-targets", "--all-features"],
        },
        CheckCommand {
            program: "cargo",
            args: &["test", "--workspace", "--all-features"],
        },
    ]
}
