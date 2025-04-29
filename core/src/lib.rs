mod video;

use std::sync::Arc;

pub use video::Video;

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("request failed: {0}")]
    RequestFailed(Arc<reqwest::Error>),
    // #[error("io operation failed: {0}")]
    // IOFailed(Arc<io::Error>),
    // #[error("docker operation failed: {0}")]
    // DockerFailed(&'static str),
    // #[error("executor failed: {0}")]
    // ExecutorFailed(&'static str),
    #[error("deserialization failed: {0}")]
    SerdeFailed(Arc<serde_json::Error>),
    // #[error("deserialization failed")]
    // DecoderFailed(Arc<decoder::Error>),
    // #[error("task join failed: {0}")]
    // JoinFailed(Arc<task::JoinError>),
    // #[error("no suitable executor was found: neither llama-server nor docker are installed")]
    // NoExecutorAvailable,
}

impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Self {
        Self::RequestFailed(Arc::new(error))
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Self::SerdeFailed(Arc::new(error))
    }
}
