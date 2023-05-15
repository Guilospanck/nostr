use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{json, Value};

use super::Error;

/// Used to send human-readable error messages
/// or other things to clients.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelayToClientCommNotice {
  pub code: String,    // "NOTICE"
  pub message: String, // NIP01 defines no rules for this message
}

impl RelayToClientCommNotice {
  /// Create new `NOTICE` message
  pub fn new_notice(message: String) -> Self {
    Self {
      code: "NOTICE".to_string(),
      message,
    }
  }

  /// Serialize as [`Value`]
  pub fn as_value(&self) -> Value {
    json!(["NOTICE", self.message])
  }

  /// Deserialize from [`Value`]
  pub fn from_value(msg: Value) -> Result<Self, Error> {
    let v = msg.as_array().ok_or(Error::InvalidData)?;

    if v.is_empty() {
      return Err(Error::InvalidData);
    }

    let v_len = v.len();

    // NOTICE
    // ["NOTICE", <message>]
    if v[0] != "NOTICE" || v_len != 2 {
      return Err(Error::InvalidData);
    }

    let message = serde_json::from_value(v[1].clone())?;
    Ok(Self::new_notice(message))
  }

  /// Get [`RelayToClientCommNotice`] as JSON string
  pub fn as_json(&self) -> String {
    self.as_value().to_string()
  }

  /// Get [`RelayToClientCommNotice`] from JSON string
  pub fn from_json<S>(msg: S) -> Result<Self, Error>
  where
    S: Into<String>,
  {
    let msg: &str = &msg.into();

    if msg.is_empty() {
      return Err(Error::InvalidData);
    }

    let value: Value = serde_json::from_str(msg)?;
    Self::from_value(value)
  }
}

impl Default for RelayToClientCommNotice {
  fn default() -> Self {
    Self {
      code: String::from("NOTICE"),
      message: String::from(""),
    }
  }
}

impl Serialize for RelayToClientCommNotice {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    let json_value: Value = self.as_value();
    json_value.serialize(serializer)
  }
}

impl<'de> Deserialize<'de> for RelayToClientCommNotice {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    // Tries to deserialize incoming thing into a json value
    let json_value: Value = Value::deserialize(deserializer)?;

    // If it succeeds, tries to deserialize it into a [`RelayToClientCommNotice`] struct
    RelayToClientCommNotice::from_value(json_value).map_err(serde::de::Error::custom)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[cfg(test)]
  use pretty_assertions::assert_eq;

  struct NoticeMock {
    mock_code: String,
    mock_message: String,
  }

  impl NoticeMock {
    fn new() -> Self {
      Self {
        mock_code: String::from("NOTICE"),
        mock_message: String::from("mock_message"),
      }
    }
  }

  #[test]
  fn test_notice_serializes_without_the_struct_key_names() {
    let mock = NoticeMock::new();
    let event = RelayToClientCommNotice {
      code: mock.mock_code.clone(),
      message: mock.mock_message.clone(),
    };

    let expected_serialized = json!([mock.mock_code, mock.mock_message]).to_string();

    assert_eq!(expected_serialized, event.as_json());
  }

  #[test]
  fn test_notice_deserializes_correctly() {
    let mock = NoticeMock::new();
    let expected_event = RelayToClientCommNotice {
      code: mock.mock_code.clone(),
      message: mock.mock_message.clone(),
    };

    let serialized = json!([mock.mock_code, mock.mock_message]).to_string();

    assert_eq!(
      RelayToClientCommNotice::from_json(serialized).unwrap(),
      expected_event
    );
  }
}
