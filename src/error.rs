use chrono::ParseError;
use kube::Error as KubeError;
use serde_json::Error as JsonError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Kubernetes Error: {0}")]
    KubeError(KubeError),
    #[error("Parsing Schedule Error: {0}")]
    ParseScheduleError(ParseError),
    #[error("JSON Error: {0}")]
    SerdeJsonError(JsonError),
    #[error("Missing object key: {0}")]
    MissingObjectKey(&'static str),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

impl From<KubeError> for Error {
    fn from(error: KubeError) -> Self {
        Error::KubeError(error)
    }
}

impl From<ParseError> for Error {
    fn from(error: ParseError) -> Self {
        Error::ParseScheduleError(error)
    }
}

impl From<JsonError> for Error {
    fn from(error: JsonError) -> Self {
        Error::SerdeJsonError(error)
    }
}
