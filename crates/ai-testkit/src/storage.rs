//! One JSON recording file per provider and scenario.

use std::{fs, io::ErrorKind, path::PathBuf};

use json_http::test_support::Recording;
use serde::{Deserialize, Serialize};

use crate::{
    error::{ErrorResultExt, Result},
    provider::Provider,
    scenario::Scenario,
};

/// Whether a response was hand-built or captured from a real provider.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RecordingSource {
    /// Realistic, credential-free fixture.
    Synthetic,
    /// A live API call observed by the recording transport.
    Recorded,
}

/// Portable input to one conformance run.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ScenarioRecording {
    /// Provenance of the exchanges.
    pub source: RecordingSource,
    /// Catalog id used to construct the adapter.
    pub catalog_model_id: String,
    /// HTTP exchanges in call order.
    pub exchanges: Vec<Recording>,
}

/// Storage boundary for the recorder and conformance tests.
#[cfg_attr(any(test, feature = "test-support"), unimock::unimock(api = RecordingStorageMock))]
pub trait RecordingStorage: Send + Sync {
    /// Loads an existing recording, or reports an absent file as `None`.
    fn load(&self, provider: Provider, scenario: Scenario) -> Result<Option<ScenarioRecording>>;
    /// Replaces one recording with formatted JSON.
    fn write(
        &self,
        provider: Provider,
        scenario: Scenario,
        recording: &ScenarioRecording,
    ) -> Result<()>;
}

/// Filesystem implementation rooted at the `recordings` directory.
pub struct FileRecordingStorage {
    root: PathBuf,
}

impl FileRecordingStorage {
    /// Selects the directory that contains provider subdirectories.
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// Resolves the stable path for one provider and scenario.
    pub fn path(&self, provider: Provider, scenario: Scenario) -> PathBuf {
        self.root
            .join(provider.name())
            .join(format!("{}.json", scenario.name()))
    }
}

impl RecordingStorage for FileRecordingStorage {
    fn load(&self, provider: Provider, scenario: Scenario) -> Result<Option<ScenarioRecording>> {
        let bytes = match fs::read(self.path(provider, scenario)) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
            error => error.context("read provider recording")?,
        };
        Ok(Some(
            serde_json::from_slice(&bytes).context("decode provider recording")?,
        ))
    }

    fn write(
        &self,
        provider: Provider,
        scenario: Scenario,
        recording: &ScenarioRecording,
    ) -> Result<()> {
        let directory = self.root.join(provider.name());
        fs::create_dir_all(&directory).context("create recording directory")?;
        let mut bytes =
            serde_json::to_vec_pretty(recording).context("encode provider recording")?;
        bytes.push(b'\n');
        fs::write(self.path(provider, scenario), bytes).context("write provider recording")?;
        Ok(())
    }
}
