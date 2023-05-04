use serde::de::Error as DeserializerError;

use serde::{ser::SerializeSeq, Deserialize, Deserializer, Serialize, Serializer};

use crate::event::Event;

/// Used to send human-readable error messages
/// or other things to clients.
///
#[derive(Debug, Serialize, Deserialize)]
pub struct RelayToClientCommNotice {
  pub code: String,    // "NOTICE"
  pub message: String, // NIP01 defines no rules for this message
}

impl RelayToClientCommNotice {
  fn as_content(&self) -> String {
    serde_json::to_string(self).unwrap()
  }
}
