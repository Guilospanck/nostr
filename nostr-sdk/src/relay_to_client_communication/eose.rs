use serde::{ser::SerializeSeq, Deserialize, Deserializer, Serialize, Serializer};

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

impl Default for RelayToClientCommEose {
  fn default() -> Self {
    Self {
      code: String::from("EOSE"),
      subscription_id: String::from(""),
    }
  }
}

impl From<RelayToClientCommEose> for Vec<String> {
  fn from(data: RelayToClientCommEose) -> Self {
    vec![data.code, data.subscription_id]
  }
}

impl<S> From<Vec<S>> for RelayToClientCommEose
where
  S: Into<String>,
{
  fn from(relay_to_client_eose: Vec<S>) -> Self {
    let relay_to_client_eose: Vec<String> =
      relay_to_client_eose.into_iter().map(|v| v.into()).collect();

    let length = relay_to_client_eose.len();

    if length == 0 || length == 1 {
      return Self::default();
    }

    Self {
      code: relay_to_client_eose[0].clone(),
      subscription_id: relay_to_client_eose[1].clone(),
    }
  }
}

impl Serialize for RelayToClientCommEose {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    // using the `impl From<RelayToClientCommEose> for Vec<String>`
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

impl<'de> Deserialize<'de> for RelayToClientCommEose {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    type Data = Vec<String>;
    // Deserializes a string (serialized) into
    // a Vec<String>
    let vec: Vec<String> = Data::deserialize(deserializer)?;
    // Then it uses the `impl<S> From<Vec<S>> for RelayToClientCommEose` to retrieve the `RelayToClientCommEose` struct
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
  }

  impl EventMock {
    fn new() -> Self {
      Self {
        mock_code: String::from("mock_code"),
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

    let expected_serialized =
      serde_json::to_string(&vec![mock.mock_code, mock.mock_subscription_id]).unwrap();

    assert_eq!(expected_serialized, event.as_content());
  }

  #[test]
  fn test_eose_deserializes_correctly() {
    let mock = EventMock::new();
    let expected_event = RelayToClientCommEose {
      code: mock.mock_code.clone(),
      subscription_id: mock.mock_subscription_id.clone(),
    };

    let serialized =
      serde_json::to_string(&vec![mock.mock_code, mock.mock_subscription_id]).unwrap();

    assert_eq!(
      RelayToClientCommEose::from_content(serialized),
      expected_event
    );
  }

  #[test]
  fn test_eose_as_vec() {
    let mock = EventMock::new();
    let event = RelayToClientCommEose {
      code: mock.mock_code.clone(),
      subscription_id: mock.mock_subscription_id.clone(),
    };

    let expected_vec = vec![mock.mock_code, mock.mock_subscription_id];

    assert_eq!(event.as_vec(), expected_vec);
  }

  #[test]
  fn test_eose_from_vec() {
    let mock = EventMock::new();
    let expected_event = RelayToClientCommEose {
      code: mock.mock_code.clone(),
      subscription_id: mock.mock_subscription_id.clone(),
    };

    let vec = vec![mock.mock_code, mock.mock_subscription_id];

    assert_eq!(RelayToClientCommEose::from_vec(vec), expected_event);
  }
}
