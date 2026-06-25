//! Workspace path discovery.

use std::env;
use std::path::PathBuf;

use crate::error::{Error, Result};

pub(crate) fn workspace_root() -> Result<PathBuf> {
    let manifest_dir = match env::var_os("CARGO_MANIFEST_DIR") {
        Some(value) => PathBuf::from(value),
        None => return Err(Error::MissingManifestDir),
    };

    match manifest_dir.parent() {
        Some(parent) => Ok(parent.to_path_buf()),
        None => Err(Error::MissingWorkspaceParent { manifest_dir }),
    }
}
