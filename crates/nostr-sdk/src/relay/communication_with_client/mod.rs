// internal modules
pub mod eose;
pub mod event;
pub mod notice;

/// [`RelayToClientCommunication`] error
#[derive(thiserror::Error, Debug)]
pub enum Error {
  /// Error serializing or deserializing JSON data
  #[error(transparent)]
  Json(#[from] serde_json::Error),
  #[error("Invalid data")]
  InvalidData
}