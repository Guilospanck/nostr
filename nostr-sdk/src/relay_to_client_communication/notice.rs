use serde::{ser::SerializeSeq, Deserialize, Deserializer, Serialize, Serializer};

/// Used to send human-readable error messages
/// or other things to clients.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelayToClientCommNotice {
  pub code: String,    // "NOTICE"
  pub message: String, // NIP01 defines no rules for this message
}

impl RelayToClientCommNotice {
  pub fn as_content(&self) -> String {
    serde_json::to_string(self).unwrap()
  }

  pub fn from_content(data: String) -> Self {
    serde_json::from_str(&data).unwrap()
  }

  pub fn from_vec(data: Vec<String>) -> Self {
    Self::from(data)
  }

  pub fn as_vec(&self) -> Vec<String> {
    self.clone().into()
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

impl From<RelayToClientCommNotice> for Vec<String> {
  fn from(data: RelayToClientCommNotice) -> Self {
    vec![data.code, data.message]
  }
}

impl<S> From<Vec<S>> for RelayToClientCommNotice
where
  S: Into<String>,
{
  fn from(relay_to_client_notice: Vec<S>) -> Self {
    let relay_to_client_notice: Vec<String> = relay_to_client_notice
      .into_iter()
      .map(|v| v.into())
      .collect();

    let length = relay_to_client_notice.len();

    if length == 0 || length == 1 {
      return Self::default();
    }

    Self {
      code: relay_to_client_notice[0].clone(),
      message: relay_to_client_notice[1].clone(),
    }
  }
}

impl Serialize for RelayToClientCommNotice {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    // using the `impl From<RelayToClientCommNotice> for Vec<String>`
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

impl<'de> Deserialize<'de> for RelayToClientCommNotice {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    type Data = Vec<String>;
    // Deserializes a string (serialized) into
    // a Vec<String>
    let vec: Vec<String> = Data::deserialize(deserializer)?;
    // Then it uses the `impl<S> From<Vec<S>> for RelayToClientCommNotice` to retrieve the `RelayToClientCommNotice` struct
    Ok(Self::from_vec(vec))
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
        mock_code: String::from("mock_code"),
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

    let expected_serialized =
      serde_json::to_string(&vec![mock.mock_code, mock.mock_message]).unwrap();

    assert_eq!(expected_serialized, event.as_content());
  }

  #[test]
  fn test_notice_deserializes_correctly() {
    let mock = NoticeMock::new();
    let expected_event = RelayToClientCommNotice {
      code: mock.mock_code.clone(),
      message: mock.mock_message.clone(),
    };

    let serialized =
      serde_json::to_string(&vec![mock.mock_code, mock.mock_message]).unwrap();

    assert_eq!(
      RelayToClientCommNotice::from_content(serialized),
      expected_event
    );
  }

  #[test]
  fn test_notice_as_vec() {
    let mock = NoticeMock::new();
    let event = RelayToClientCommNotice {
      code: mock.mock_code.clone(),
      message: mock.mock_message.clone(),
    };

    let expected_vec = vec![mock.mock_code, mock.mock_message];

    assert_eq!(event.as_vec(), expected_vec);
  }

  #[test]
  fn test_notice_from_vec() {
    let mock = NoticeMock::new();
    let expected_event = RelayToClientCommNotice {
      code: mock.mock_code.clone(),
      message: mock.mock_message.clone(),
    };

    let vec = vec![mock.mock_code, mock.mock_message];

    assert_eq!(RelayToClientCommNotice::from_vec(vec), expected_event);
  }
}
