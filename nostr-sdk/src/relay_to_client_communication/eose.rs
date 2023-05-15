use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{json, Value};

use super::Error;

/// Used to indicate the End Of Stored Events (EOSE)
/// and the beginning of events newly received in
/// real-time.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelayToClientCommEose {
  pub code: String, // "EOSE"
  pub subscription_id: String,
}

impl RelayToClientCommEose {
  // Create new `EOSE` message
  pub fn new_eose(subscription_id: String) -> Self {
    Self {
      code: "EOSE".to_string(),
      subscription_id,
    }
  }

  pub fn as_value(&self) -> Value {
    json!(["EOSE", self.subscription_id])
  }

  pub fn from_value(msg: Value) -> Result<Self, Error> {
    let v = msg.as_array().ok_or(Error::InvalidData)?;

    if v.is_empty() {
      return Err(Error::InvalidData);
    }

    let v_len = v.len();

    // EOSE
    // ["EOSE", <subscription_id>]
    if v[0] != "EOSE" || v_len != 2 {
      return Err(Error::InvalidData);
    }

    let subscription_id = serde_json::from_value(v[1].clone())?;
    Ok(Self::new_eose(subscription_id))
  }

  /// Get [`RelayToClientCommEose`] as JSON string
  pub fn as_json(&self) -> String {
    self.as_value().to_string()
  }

  /// Get [`RelayToClientCommEose`] from JSON
  pub fn from_json<S>(msg: S) -> Result<Self, Error>
  where
    S: Into<String>,
  {
    let msg: &str = &msg.into();

    if msg.is_empty() {
      return Err(Error::InvalidData);
    }

    let json_value: Value = serde_json::from_str(msg)?;
    Self::from_value(json_value)
  }
}

impl Default for RelayToClientCommEose {
  fn default() -> Self {
    Self {
      code: String::from("EOSE"),
      subscription_id: String::from(""),
    }
  }
}

impl Serialize for RelayToClientCommEose {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    let json_value: Value = self.as_value();
    json_value.serialize(serializer)
  }
}

impl<'de> Deserialize<'de> for RelayToClientCommEose {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    let json_value: Value = Value::deserialize(deserializer)?;
    RelayToClientCommEose::from_value(json_value).map_err(serde::de::Error::custom)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[cfg(test)]
  use pretty_assertions::assert_eq;

  struct EventMock {
    mock_code: String,
    mock_subscription_id: String,
  }

  impl EventMock {
    fn new() -> Self {
      Self {
        mock_code: String::from("EOSE"),
        mock_subscription_id: String::from("mock_subscription_id"),
      }
    }
  }

  #[test]
  fn test_eose_serializes_without_the_struct_key_names() {
    let mock = EventMock::new();
    let event = RelayToClientCommEose {
      code: mock.mock_code.clone(),
      subscription_id: mock.mock_subscription_id.clone(),
    };

    let expected_serialized = json!([mock.mock_code, mock.mock_subscription_id]).to_string();

    assert_eq!(expected_serialized, event.as_json());
  }

  #[test]
  fn test_eose_deserializes_correctly() {
    let mock = EventMock::new();
    let expected_event = RelayToClientCommEose {
      code: mock.mock_code.clone(),
      subscription_id: mock.mock_subscription_id.clone(),
    };

    let serialized = json!([mock.mock_code, mock.mock_subscription_id]).to_string();

    assert_eq!(
      RelayToClientCommEose::from_json(serialized).unwrap(),
      expected_event
    );
  }
}
