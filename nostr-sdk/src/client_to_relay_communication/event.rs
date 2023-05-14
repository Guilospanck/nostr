use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{json, Value};

use crate::event::Event;

use super::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientToRelayCommEvent {
  pub code: String, // "EVENT"
  pub event: Event,
}

impl ClientToRelayCommEvent {
  pub fn new_event(event: Event) -> Self {
    Self {
      code: "EVENT".to_string(),
      event,
    }
  }

  /// Get event communication as JSON string
  pub fn as_json(&self) -> String {
    self.as_value().to_string()
  }

  /// Deserialize [`ClientToRelayCommEvent`] from JSON string
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

  /// Serialize as [`Value`]
  pub fn as_value(&self) -> Value {
    json!(["EVENT", self.event])
  }

  /// Deserialize from [`Value`]
  pub fn from_value(msg: Value) -> Result<Self, Error> {
    let v = msg.as_array().ok_or(Error::InvalidData)?;

    if v.is_empty() {
      return Err(Error::InvalidData);
    }

    let v_len: usize = v.len();

    // Event
    // ["EVENT", <event JSON>]
    if v[0] != "EVENT" || v_len != 2 {
      return Err(Error::InvalidData);
    }

    let event: Event = serde_json::from_value(v[1].clone())?;
    Ok(Self::new_event(event))
  }
}

impl Default for ClientToRelayCommEvent {
  fn default() -> Self {
    Self {
      code: String::from("EVENT"),
      event: Event::default(),
    }
  }
}

impl Serialize for ClientToRelayCommEvent {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    let json_value: Value = self.as_value();
    json_value.serialize(serializer)
  }
}

impl<'de> Deserialize<'de> for ClientToRelayCommEvent {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    // We don't know what we're receiving. So just try to deserialize it
    // to some value
    let json_value: Value = Value::deserialize(deserializer)?;

    // If the deserialization happens correctly (i.e.: is a valid JSON),
    // We verify if this JSON is the one we want, namely `ClientToRelayCommEvent`
    ClientToRelayCommEvent::from_value(json_value).map_err(serde::de::Error::custom)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[cfg(test)]
  use pretty_assertions::assert_eq;

  struct EvtSut {
    mock_event: Event,
    mock_client_event: ClientToRelayCommEvent,
  }

  impl EvtSut {
    fn new() -> Self {
      let mock_filter_id = String::from("05b25af3-4250-4fbf-8ef5-97220858f9ab");

      let mock_event = Self::mock_event(mock_filter_id);

      let mock_client_event = ClientToRelayCommEvent {
        code: "EVENT".to_string(),
        event: mock_event.clone(),
      };

      Self {
        mock_event,
        mock_client_event,
      }
    }

    fn mock_event(id: String) -> Event {
      Event {
        id,
        ..Default::default()
      }
    }
  }

  #[test]
  fn test_client_to_relay_comm_event_default() {
    let expected = ClientToRelayCommEvent {
      code: "EVENT".to_owned(),
      event: Event::default(),
    };

    let result = ClientToRelayCommEvent::default();

    assert_eq!(expected, result);
  }

  #[test]
  fn test_client_to_relay_comm_event_as_json() {
    let mock = EvtSut::new();

    let event_as_str = mock.mock_event.as_json();
    let expected =
      ClientToRelayCommEvent::from_json(format!(r#"["EVENT",{}]"#, event_as_str)).unwrap();

    let result_as_json = mock.mock_client_event.as_json();
    let result = ClientToRelayCommEvent::from_json(result_as_json).unwrap();

    assert_eq!(expected, result);
  }

  #[test]
  fn test_client_to_relay_comm_event_from_json() {
    let mock = EvtSut::new();

    let event_json = mock.mock_event.as_value();
    let from_json = json!(["EVENT", event_json]).to_string();

    let result = ClientToRelayCommEvent::from_json(from_json).unwrap();

    assert_eq!(result, mock.mock_client_event);
  }
}
