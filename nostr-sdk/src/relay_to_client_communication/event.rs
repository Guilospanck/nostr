use serde::{ser::SerializeSeq, Deserialize, Deserializer, Serialize, Serializer};

use crate::event::Event;

/// Used to send events requests by clients.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelayToClientCommEvent {
  pub code: String, // "EVENT"
  pub subscription_id: String,
  pub event: Event,
}

impl RelayToClientCommEvent {
  pub fn as_content(&self) -> String {
    serde_json::to_string(self).unwrap()
  }

  pub fn from_content(content: String) -> Self {
    serde_json::from_str(&content).unwrap()
  }

  pub fn as_vec(&self) -> Vec<String> {
    self.clone().into()
  }

  pub fn from_vec(data: Vec<String>) -> Self {
    Self::from(data)
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

impl From<RelayToClientCommEvent> for Vec<String> {
  fn from(data: RelayToClientCommEvent) -> Self {
    vec![data.code, data.subscription_id, data.event.as_str()]
  }
}

impl<S> From<Vec<S>> for RelayToClientCommEvent
where
  S: Into<String>,
{
  fn from(relay_to_client_event: Vec<S>) -> Self {
    let relay_to_client_event: Vec<String> = relay_to_client_event
      .into_iter()
      .map(|v| v.into())
      .collect();

    let length = relay_to_client_event.len();

    if length == 0 || length == 1 {
      return Self::default();
    } else if length == 2 {
      return Self {
        subscription_id: relay_to_client_event[1].clone(),
        ..Default::default()
      };
    }

    Self {
      code: relay_to_client_event[0].clone(),
      subscription_id: relay_to_client_event[1].clone(),
      event: Event::from_serialized(&relay_to_client_event[2]),
    }
  }
}

impl Serialize for RelayToClientCommEvent {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    // using the `impl From<RelayToClientCommEvent> for Vec<String>`
    let data: Vec<String> = self.as_vec();
    // A Vec<_> is a sequence, therefore we must tell the
    // deserializer how long is the sequence (vector's length)
    let mut seq = serializer.serialize_seq(Some(data.len()))?;
    // Serialize each element of the Vector
    for element in data.into_iter() {
      seq.serialize_element(&element)?;
    }
    // Finalize the serialization and return the result
    seq.end()
  }
}

impl<'de> Deserialize<'de> for RelayToClientCommEvent {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    type Data = Vec<String>;
    // Deserializes a string (serialized) into
    // a Vec<String>
    let vec: Vec<String> = Data::deserialize(deserializer)?;
    // Then it uses the `impl<S> From<Vec<S>> for RelayToClientCommEvent` to retrieve the `RelayToClientCommEvent` struct
    Ok(Self::from_vec(vec))
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
        mock_code: String::from("mock_code"),
        mock_subscription_id: String::from("mock_subscription_id"),
        mock_event: Event::default(),
      }
    }
  }

  #[test]
  fn test_event_serializes_without_the_struct_key_names() {
    let mock = EventMock::new();
    let event = RelayToClientCommEvent {
      code: mock.mock_code.clone(),
      subscription_id: mock.mock_subscription_id.clone(),
      event: mock.mock_event.clone(),
    };

    let expected_serialized = serde_json::to_string(&vec![
      mock.mock_code,
      mock.mock_subscription_id,
      mock.mock_event.as_str(),
    ])
    .unwrap();

    assert_eq!(expected_serialized, event.as_content());
  }

  #[test]
  fn test_event_deserializes_correctly() {
    let mock = EventMock::new();
    let expected_event = RelayToClientCommEvent {
      code: mock.mock_code.clone(),
      subscription_id: mock.mock_subscription_id.clone(),
      event: mock.mock_event.clone(),
    };

    let serialized = serde_json::to_string(&vec![
      mock.mock_code,
      mock.mock_subscription_id,
      mock.mock_event.as_str(),
    ])
    .unwrap();

    assert_eq!(RelayToClientCommEvent::from_content(serialized), expected_event);
  }

  #[test]
  fn test_event_as_vec() {
    let mock = EventMock::new();
    let event = RelayToClientCommEvent {
      code: mock.mock_code.clone(),
      subscription_id: mock.mock_subscription_id.clone(),
      event: mock.mock_event.clone(),
    };

    let expected_vec = vec![
      mock.mock_code,
      mock.mock_subscription_id,
      mock.mock_event.as_str(),
    ];

    assert_eq!(event.as_vec(), expected_vec);
  }

  #[test]
  fn test_event_from_vec() {
    let mock = EventMock::new();
    let expected_event = RelayToClientCommEvent {
      code: mock.mock_code.clone(),
      subscription_id: mock.mock_subscription_id.clone(),
      event: mock.mock_event.clone(),
    };

    let vec = vec![
      mock.mock_code,
      mock.mock_subscription_id,
      mock.mock_event.as_str(),
    ];

    assert_eq!(RelayToClientCommEvent::from_vec(vec), expected_event);
  }
}
