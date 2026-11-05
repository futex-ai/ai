//! Explicit file output for the capability reference command.

use std::{fs, path::Path};

use crate::error::{Error, Result};

use super::{catalogs::model_specs, document::replace_matrix, render::render_matrix};

/// Writes the generated capability table while preserving surrounding prose.
pub(crate) trait CapabilityMatrixGenerator {
    /// Updates the model-capabilities document of the supplied workspace.
    fn generate(&self, root: &Path) -> Result<()>;
}

/// Filesystem implementation used by the command-line composition root.
pub(crate) struct FileCapabilityMatrixGenerator;

impl CapabilityMatrixGenerator for FileCapabilityMatrixGenerator {
    fn generate(&self, root: &Path) -> Result<()> {
        let path = root.join("docs/protocol/model-capabilities.md");
        let document = match fs::read_to_string(&path) {
            Ok(document) => document,
            Err(source) => return Err(Error::MatrixRead { path, source }),
        };
        let output = replace_matrix(&document, &render_matrix(&model_specs()))?;
        if output != document {
            match fs::write(&path, output) {
                Ok(()) => {}
                Err(source) => return Err(Error::MatrixWrite { path, source }),
            }
        }
        Ok(())
    }
}
