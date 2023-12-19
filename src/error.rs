//! Error types
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Unable to draw: {0}")]
    DrawError(skrifa::outline::DrawError),
}
