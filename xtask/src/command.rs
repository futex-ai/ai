//! External command execution helpers.

use std::path::Path;
use std::process::Command;

use crate::error::{Error, Result};

pub(crate) fn run(root: &Path, program: &str, args: &[&str]) -> Result<()> {
    let command_label = command_label(program, args);
    let status = match Command::new(program).args(args).current_dir(root).status() {
        Ok(status) => status,
        Err(source) => {
            return Err(Error::CommandStart {
                command: command_label,
                source,
            });
        }
    };

    if status.success() {
        return Ok(());
    }

    Err(Error::CommandFailed {
        command: command_label,
        status,
    })
}

pub(crate) fn command_label(program: &str, args: &[&str]) -> String {
    let mut parts = vec![program.to_owned()];
    parts.extend(args.iter().map(|arg| (*arg).to_owned()));
    parts.join(" ")
}
