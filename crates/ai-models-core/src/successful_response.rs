//! Checked successful responses shared by native parsers and downloads.

use std::fmt;

/// A buffered response that passed the shared 2xx success boundary.
pub struct SuccessfulResponse<T> {
    response: T,
}

impl<T> SuccessfulResponse<T> {
    /// Keeps successful responses and returns all others intact for status classification.
    pub fn check(status: u16, response: T) -> Result<Self, T> {
        if (200..300).contains(&status) {
            Ok(Self { response })
        } else {
            Err(response)
        }
    }

    /// Releases a response to its native success parser or binary consumer.
    pub fn into_inner(self) -> T {
        self.response
    }
}

impl<T> fmt::Debug for SuccessfulResponse<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SuccessfulResponse")
            .finish_non_exhaustive()
    }
}
