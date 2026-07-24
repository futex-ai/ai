//! Model-visible tool output envelope contracts.

mod envelope;
mod error;
mod id;
mod inline;
mod read_request;
mod reason;
mod window;

pub use envelope::ToolOutputEnvelope;
pub use error::{ToolOutputEnvelopeError, ToolOutputEnvelopeResult};
pub use id::ToolOutputId;
pub use inline::ToolOutputInlineEnvelope;
pub use read_request::ToolOutputReadRequest;
pub use reason::ToolOutputRemainderUnavailableReason;
pub use window::ToolOutputWindowEnvelope;

pub(super) const INLINE_TYPE: &str = "tool_output";
pub(super) const WINDOW_TYPE: &str = "tool_output_window";
