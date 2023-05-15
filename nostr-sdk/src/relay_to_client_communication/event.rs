use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{json, Value};

use crate::event::Event;

use super::Error;

/// Used to send events requests by clients.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelayToClientCommEvent {
  pub code: String, // "EVENT"
  pub subscription_id: String,
  pub event: Event,
}

impl RelayToClientCommEvent {
  /// Create new [`RelayToClientCommEvent`] message
  pub fn new_event(subscription_id: String, event: Event) -> Self {
    Self {
      code: "EVENT".to_string(),
      subscription_id,
      event,
    }
  }

  /// Serialize as [`Value`]
  pub fn as_value(&self) -> Value {
    json!(["EVENT", self.subscription_id, self.event])
  }

  /// Deserialize from [`Value`]
  pub fn from_value(msg: Value) -> Result<Self, Error> {
    let v = msg.as_array().ok_or(Error::InvalidData)?;

    if v.is_empty() {
      return Err(Error::InvalidData);
    }

    let v_len: usize = v.len();

    // Event
    // ["EVENT", <subscription_id>, <event JSON>]
    if v[0] != "EVENT" || v_len != 3 {
      return Err(Error::InvalidData);
    }

    let subscription_id = serde_json::from_value(v[1].clone())?;
    let event: Event = serde_json::from_value(v[2].clone())?;
    Ok(Self::new_event(subscription_id, event))
  }

  /// Get [`RelayToClientCommEvent`] as JSON string
  pub fn as_json(&self) -> String {
    self.as_value().to_string()
  }

  /// Get [`RelayToClientCommEvent`] from JSON string
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

impl Default for RelayToClientCommEvent {
  fn default() -> Self {
    Self {
      code: String::from("EVENT"),
      subscription_id: String::from(""),
      event: Event::default(),
    }
  }
}

impl Serialize for RelayToClientCommEvent {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    let json_value: Value = self.as_value();
    json_value.serialize(serializer)
  }
}

impl<'de> Deserialize<'de> for RelayToClientCommEvent {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    // Tries to deserialize into a `Value`
    let json_value: Value = Value::deserialize(deserializer)?;

    // Knowing the `Value`, verifies if it of type `RelayToClientCommEvent`
    RelayToClientCommEvent::from_value(json_value).map_err(serde::de::Error::custom)
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
    mock_event: Event,
  }

  impl EventMock {
    fn new() -> Self {
      Self {
        mock_code: String::from("EVENT"),
        mock_subscription_id: String::from("mock_subscription_id"),
        mock_event: Event::default(),
      }
    }
  }

  #[test]
  fn test_event_serializes_without_the_struct_key_names() {
    let mock = EventMock::new();
    let event =
      RelayToClientCommEvent::new_event(mock.mock_subscription_id.clone(), mock.mock_event);
    let expected_serialized =
      json!(["EVENT", mock.mock_subscription_id, Event::default()]).to_string();

    assert_eq!(expected_serialized, event.as_json());
  }

  #[test]
  fn test_event_deserializes_correctly() {
    let mock = EventMock::new();
    let expected_event = RelayToClientCommEvent {
      code: mock.mock_code.clone(),
      subscription_id: mock.mock_subscription_id.clone(),
      event: mock.mock_event.clone(),
    };

    let serialized =
      json!([mock.mock_code, mock.mock_subscription_id, mock.mock_event,]).to_string();

    assert_eq!(
      RelayToClientCommEvent::from_json(serialized).unwrap(),
      expected_event
    );
  }
}
