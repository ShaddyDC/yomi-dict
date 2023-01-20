use thiserror::Error;
use zip::result::ZipError;

#[derive(Error, Debug)]
pub enum YomiDictError {
    #[error("An IO error occured: `{0}`")]
    Io(std::io::Error),
    #[error("Archive is invalid: `{0}`")]
    InvalidArchive(&'static str),
    #[error("Archive is not supported: `{0}`")]
    UnsupportedArchive(&'static str),
    #[error("File index.json not found in archive")]
    IndexNotFound,
    #[error("Error parsing Json: `{0}`")]
    JsonError(serde_json::Error),
    #[error("Error parsing JSObject: `{0}`")]
    JsobjError(serde_wasm_bindgen::Error),
    #[error("Error with storage: `{0}`")]
    StorageError(rexie::Error),
}

impl From<rexie::Error> for YomiDictError {
    fn from(e: rexie::Error) -> Self {
        Self::StorageError(e)
    }
}

impl From<serde_json::Error> for YomiDictError {
    fn from(e: serde_json::Error) -> Self {
        Self::JsonError(e)
    }
}

impl From<serde_wasm_bindgen::Error> for YomiDictError {
    fn from(e: serde_wasm_bindgen::Error) -> Self {
        Self::JsobjError(e)
    }
}

impl From<ZipError> for YomiDictError {
    fn from(e: ZipError) -> Self {
        match e {
            ZipError::InvalidArchive(s) => Self::InvalidArchive(s),
            ZipError::UnsupportedArchive(s) => Self::UnsupportedArchive(s),
            ZipError::Io(e) => Self::Io(e),
            ZipError::FileNotFound => Self::UnsupportedArchive("Unknown error occured"),
        }
    }
}
