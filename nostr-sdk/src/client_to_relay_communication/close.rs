use serde::{de, ser::SerializeSeq, Deserialize, Deserializer, Serialize, Serializer};

use super::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientToRelayCommClose {
  pub code: String, // "CLOSE"
  pub subscription_id: String,
}

impl ClientToRelayCommClose {
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

impl Default for ClientToRelayCommClose {
  fn default() -> Self {
    Self {
      code: String::from("CLOSE"),
      subscription_id: String::from(""),
    }
  }
}

impl<S> TryFrom<Vec<S>> for ClientToRelayCommClose
where
  S: Into<String>,
{
  type Error = Error;

  fn try_from(data: Vec<S>) -> Result<Self, Self::Error> {
    let data: Vec<String> = data.into_iter().map(|v| v.into()).collect();
    let data_len: usize = data.len();

    if data_len != 2 || data[0] != *"CLOSE" {
      return Err(Error::InvalidData);
    }

    Ok(Self {
      code: data[0].clone(),
      subscription_id: data[1].clone(),
    })
  }
}

impl From<ClientToRelayCommClose> for Vec<String> {
  fn from(value: ClientToRelayCommClose) -> Self {
    vec![value.code, value.subscription_id]
  }
}

impl Serialize for ClientToRelayCommClose {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    // using the `impl From<ClientToRelayCommClose> for Vec<String>`
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

impl<'de> Deserialize<'de> for ClientToRelayCommClose {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    type Data = Vec<String>;
    // Deserializes a string (serialized) into
    // a Vec<String>
    let vec: Vec<String> = Data::deserialize(deserializer)?;
    // Then it uses the `impl<S> From<Vec<S>> for ClientToRelayCommClose` to retrieve the `ClientToRelayCommClose` struct
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

  struct CloseSut {
    mock_client_close: ClientToRelayCommClose,
  }

  impl CloseSut {
    fn new() -> Self {
      let mock_subscription_id = "mock_subscription_id".to_string();

      let mock_client_close = ClientToRelayCommClose {
        code: "CLOSE".to_string(),
        subscription_id: mock_subscription_id,
      };

      Self { mock_client_close }
    }
  }

  #[test]
  fn test_client_to_relay_comm_close_default() {
    let expected = ClientToRelayCommClose {
      code: "CLOSE".to_owned(),
      subscription_id: "".to_owned(),
    };

    let result = ClientToRelayCommClose::default();

    assert_eq!(expected, result);
  }

  #[test]
  fn test_client_to_relay_comm_close_as_str() {
    let mock = CloseSut::new();

    let client_close2 = ClientToRelayCommClose::default();

    let expected = r#"["CLOSE","mock_subscription_id"]"#.to_owned();
    let expected2 = r#"["CLOSE",""]"#.to_owned();

    assert_eq!(expected, mock.mock_client_close.as_str().unwrap());
    assert_eq!(expected2, client_close2.as_str().unwrap());
  }

  #[test]
  fn test_client_to_relay_comm_close_from_str() {
    let mock = CloseSut::new();

    let expected = "[\"CLOSE\",\"mock_subscription_id\"]".to_owned();
    let expected2 = "[\"CLOSE\",\"\"]".to_owned();
    let expected3 = "[\"\",\"\"]".to_owned();
    let expected4 = "[\"\"]".to_owned();
    let expected5 = "[]".to_owned();

    let result = ClientToRelayCommClose::from_string(expected).unwrap();
    let result2 = ClientToRelayCommClose::from_string(expected2).unwrap();
    let result3 = ClientToRelayCommClose::from_string(expected3);
    let result4 = ClientToRelayCommClose::from_string(expected4);
    let result5 = ClientToRelayCommClose::from_string(expected5);

    let client_close2 = ClientToRelayCommClose::default();

    assert_eq!(result, mock.mock_client_close);
    assert_eq!(result2, client_close2);
    assert!(result3.is_err());
    assert!(result4.is_err());
    assert!(result5.is_err());
  }

  #[test]
  fn test_client_to_relay_comm_close_from_vec() {
    let mock = CloseSut::new();

    let expected: Vec<String> = vec!["CLOSE".to_owned(), "mock_subscription_id".to_owned()];
    let expected2: Vec<String> = vec!["CLOSE".to_owned(), "".to_owned()];
    let expected3: Vec<String> = vec!["CLOSE".to_owned()];
    let expected4: Vec<String> = vec!["".to_owned()];
    let expected5: Vec<String> = vec![];

    let result = ClientToRelayCommClose::from_vec(expected).unwrap();
    let result2 = ClientToRelayCommClose::from_vec(expected2).unwrap();
    let result3 = ClientToRelayCommClose::from_vec(expected3);
    let result4 = ClientToRelayCommClose::from_vec(expected4);
    let result5 = ClientToRelayCommClose::from_vec(expected5);

    let default_client_close = ClientToRelayCommClose::default();

    assert_eq!(result, mock.mock_client_close);
    assert_eq!(result2, default_client_close);
    assert!(result3.is_err());
    assert!(result4.is_err());
    assert!(result5.is_err());
  }

  #[test]
  fn test_client_to_relay_comm_close_as_vec() {
    let mock = CloseSut::new();

    let expected: Vec<String> = vec!["CLOSE".to_owned(), "mock_subscription_id".to_owned()];
    let expected2: Vec<String> = vec!["CLOSE".to_owned(), "".to_owned()];

    let default_client_close = ClientToRelayCommClose::default();

    assert_eq!(expected, mock.mock_client_close.as_vec());
    assert_eq!(expected2, default_client_close.as_vec());
  }
}
