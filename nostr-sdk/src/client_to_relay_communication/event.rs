use serde::{de, ser::SerializeSeq, Deserialize, Deserializer, Serialize, Serializer};

use crate::event::Event;

use super::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientToRelayCommEvent {
  pub code: String, // "EVENT"
  pub event: Event,
}

impl ClientToRelayCommEvent {
  pub fn as_str(&self) -> Result<String, Error> {
    serde_json::to_string(self).map_err(Error::Json)
  }

  pub fn from_string(data: String) -> Result<Self, Error> {
    serde_json::from_str(&data).map_err(Error::Json)
  }

  pub fn as_vec(&self) -> Vec<String> {
    self.clone().into()
  }

  pub fn from_vec(data: Vec<String>) -> Result<Self, Error> {
    Self::try_from(data)
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

impl From<ClientToRelayCommEvent> for Vec<String> {
  fn from(data: ClientToRelayCommEvent) -> Self {
    vec![data.code, data.event.as_str()]
  }
}

impl<S> TryFrom<Vec<S>> for ClientToRelayCommEvent
where
  S: Into<String>,
{
  type Error = Error;

  fn try_from(data: Vec<S>) -> Result<Self, Self::Error> {
    let data: Vec<String> = data.into_iter().map(|v| v.into()).collect();
    let data_len: usize = data.len();

    if data_len != 2 || data[0] != *"EVENT" {
      return Err(Error::InvalidData);
    }

    Ok(Self {
      code: data[0].clone(),
      event: Event::from_serialized(data[1].clone().as_str()),
    })
  }
}

impl Serialize for ClientToRelayCommEvent {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    // using the `impl From<ClientToRelayCommEvent> for Vec<String>`
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

impl<'de> Deserialize<'de> for ClientToRelayCommEvent {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    type Data = Vec<String>;
    // Deserializes a string (serialized) into
    // a Vec<String>
    let vec: Vec<String> = Data::deserialize(deserializer)?;
    // Then it uses the `impl<S> From<Vec<S>> for ClientToRelayCommEvent` to retrieve the `ClientToRelayCommEvent` struct
    let result = Self::from_vec(vec);
    if result.is_err() {
      return Err(Error::InvalidData).map_err(de::Error::custom);
    }
    Ok(result.unwrap())
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
  fn test_client_to_relay_comm_event_as_str() {
    let mock = EvtSut::new();

    let event_as_str = mock.mock_event.as_str();

    let expected = format!(r#"["EVENT","{}"]"#, event_as_str);

    assert_eq!(
      expected,
      mock
        .mock_client_event
        .as_str()
        .unwrap()
        .replace("\\\"", "\"")
    );
  }

  #[test]
  fn test_client_to_relay_comm_event_from_str() {
    let mock = EvtSut::new();

    let expected = "[\"EVENT\",\"{\\\"id\\\":\\\"05b25af3-4250-4fbf-8ef5-97220858f9ab\\\",\\\"pubkey\\\":\\\"\\\",\\\"created_at\\\":0,\\\"kind\\\":1,\\\"tags\\\":[],\\\"content\\\":\\\"\\\",\\\"sig\\\":\\\"\\\"}\"]".to_owned();

    let result = ClientToRelayCommEvent::from_string(expected).unwrap();

    assert_eq!(result, mock.mock_client_event);
  }

  #[test]
  fn test_client_to_relay_comm_event_from_vec() {
    let mock = EvtSut::new();

    let expected: Vec<String> = vec!["EVENT".to_owned(), mock.mock_event.as_str()];
    let expected2: Vec<String> = vec!["EVENT".to_owned()];
    let expected3: Vec<String> = vec![];

    let result = ClientToRelayCommEvent::from_vec(expected).unwrap();
    let result2 = ClientToRelayCommEvent::from_vec(expected2);
    let result3 = ClientToRelayCommEvent::from_vec(expected3);

    assert_eq!(result, mock.mock_client_event);
    assert!(result2.is_err());
    assert!(result3.is_err());
  }

  #[test]
  fn test_client_to_relay_comm_event_as_vec() {
    let mock = EvtSut::new();

    let default_client_to_relay_event = ClientToRelayCommEvent::default();

    let expected_default: Vec<String> = vec!["EVENT".to_owned(), Event::default().as_str()];
    let expected: Vec<String> = vec!["EVENT".to_owned(), mock.mock_event.as_str()];

    assert_eq!(expected_default, default_client_to_relay_event.as_vec());
    assert_eq!(expected, mock.mock_client_event.as_vec());
  }
}
