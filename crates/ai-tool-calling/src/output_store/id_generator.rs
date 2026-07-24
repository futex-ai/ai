//! Fallible output id generation.

use std::sync::Arc;
use std::time::{SystemTime, SystemTimeError, UNIX_EPOCH};

use ai_interface::ToolOutputId;
use thiserror::Error;
use uuid::Builder;

const MAX_UUID_V7_TIMESTAMP_MILLIS: u128 = (1_u128 << 48) - 1;

/// Generates opaque identifiers for retained tool output.
#[cfg_attr(test, unimock::unimock(api = [ToolOutputIdGeneratorMock]))]
pub(crate) trait ToolOutputIdGenerator: Send + Sync {
    /// Generates one opaque UUIDv7-backed output id.
    fn generate(&self) -> OutputIdGenerationResult<ToolOutputId>;
}

/// Shared dynamic output-id generator.
pub(crate) type DynToolOutputIdGenerator = Arc<dyn ToolOutputIdGenerator>;

/// UUIDv7 generator backed by the system clock and operating-system entropy.
pub(crate) struct SystemToolOutputIdGenerator;

impl ToolOutputIdGenerator for SystemToolOutputIdGenerator {
    fn generate(&self) -> OutputIdGenerationResult<ToolOutputId> {
        let elapsed = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(elapsed) => elapsed,
            Err(source) => return Err(OutputIdGenerationError::Clock { source }),
        };
        let millis = elapsed.as_millis();
        if millis > MAX_UUID_V7_TIMESTAMP_MILLIS {
            return Err(OutputIdGenerationError::TimestampOutOfRange { millis });
        }
        let millis = match u64::try_from(millis) {
            Ok(millis) => millis,
            Err(_) => return Err(OutputIdGenerationError::TimestampOutOfRange { millis }),
        };
        let mut random_bytes = [0_u8; 10];
        if let Err(source) = getrandom::fill(&mut random_bytes) {
            return Err(OutputIdGenerationError::Entropy { source });
        }
        let uuid = Builder::from_unix_timestamp_millis(millis, &random_bytes).into_uuid();
        Ok(ToolOutputId::from_opaque(format!("toolout_{uuid}")))
    }
}

#[derive(Debug, Error)]
/// Failures raised while generating a UUIDv7 output id.
pub(crate) enum OutputIdGenerationError {
    /// The system clock reported a time before the Unix epoch.
    #[error(
        "[ai_tool_calling/output_store/id_generator] system clock predates Unix epoch: {source}"
    )]
    Clock {
        /// Underlying system clock failure.
        #[source]
        source: SystemTimeError,
    },
    /// The system clock exceeded UUIDv7's 48-bit millisecond range.
    #[error(
        "[ai_tool_calling/output_store/id_generator] Unix timestamp {millis}ms exceeds the UUIDv7 range"
    )]
    TimestampOutOfRange {
        /// Millisecond timestamp that could not be encoded.
        millis: u128,
    },
    /// The operating system could not provide random bytes.
    #[error("[ai_tool_calling/output_store/id_generator] failed to acquire entropy: {source}")]
    Entropy {
        /// Underlying entropy-source failure.
        #[source]
        source: getrandom::Error,
    },
}

/// Result alias for output-id generation.
pub(crate) type OutputIdGenerationResult<T> = std::result::Result<T, OutputIdGenerationError>;
