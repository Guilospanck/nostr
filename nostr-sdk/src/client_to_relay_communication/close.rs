use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{json, Value};

use super::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientToRelayCommClose {
  pub code: String, // "CLOSE"
  pub subscription_id: String,
}

impl ClientToRelayCommClose {
  pub fn new_close(subscription_id: String) -> Self {
    Self {
      code: "CLOSE".to_string(),
      subscription_id,
    }
  }

  /// Serialize as [`Value`]
  pub fn as_value(&self) -> Value {
    json!(["CLOSE", self.subscription_id])
  }

  /// Deserialize from [`Value`]
  pub fn from_value(msg: Value) -> Result<Self, Error> {
    let v = msg.as_array().ok_or(Error::InvalidData)?;

    if v.is_empty() {
      return Err(Error::InvalidData);
    }

    let v_len: usize = v.len();

    // Close
    // ["CLOSE", subscription_id]
    if v[0] != "CLOSE" || v_len != 2 {
      return Err(Error::InvalidData);
    }
    
    let subscription_id = serde_json::from_value(v[1].clone())?;
    Ok(Self::new_close(subscription_id))
  }

  /// Get [`ClientToRelayCommClose`] as JSON string
  pub fn as_json(&self) -> String {
    self.as_value().to_string()
  }

  /// Deserialize [`ClientToRelayCommClose`] from JSON string
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

impl Default for ClientToRelayCommClose {
  fn default() -> Self {
    Self {
      code: String::from("CLOSE"),
      subscription_id: String::from(""),
    }
  }
}

impl Serialize for ClientToRelayCommClose {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    let json_value: Value = self.as_value();
    json_value.serialize(serializer)
  }
}

impl<'de> Deserialize<'de> for ClientToRelayCommClose {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    // As it usually happens with deserialization, first we need to deserialize it
    // to something that we know how to deal with. In this case, we know how to
    // deal with "Value" types. Therefore, we use its deserializer.
    let json_value: Value = Value::deserialize(deserializer)?;

    // Now that we have the "Value", we can verify if it abides in the structure
    // we are working on, namely `ClientToRelayCommClose`
    ClientToRelayCommClose::from_value(json_value).map_err(serde::de::Error::custom)
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
  fn test_client_to_relay_comm_close_as_json() {
    let mock = CloseSut::new();

    let client_close2 = ClientToRelayCommClose::default();

    let expected = r#"["CLOSE","mock_subscription_id"]"#.to_owned();
    let expected2 = r#"["CLOSE",""]"#.to_owned();

    assert_eq!(expected, mock.mock_client_close.as_json());
    assert_eq!(expected2, client_close2.as_json());
  }

  #[test]
  fn test_client_to_relay_comm_close_from_json() {
    let mock = CloseSut::new();

    let from_json = json!(["CLOSE", "mock_subscription_id"]).to_string();
    let from_json2 = json!(["CLOSE", ""]).to_string();
    let from_json3 = json!(["", ""]).to_string();
    let from_json4 = json!([""]).to_string();
    let from_json5 = json!([]).to_string();

    let result = ClientToRelayCommClose::from_json(from_json).unwrap();
    let result2 = ClientToRelayCommClose::from_json(from_json2).unwrap();
    let result3 = ClientToRelayCommClose::from_json(from_json3);
    let result4 = ClientToRelayCommClose::from_json(from_json4);
    let result5 = ClientToRelayCommClose::from_json(from_json5);

    let client_close2 = ClientToRelayCommClose::default();

    assert_eq!(result, mock.mock_client_close);
    assert_eq!(result2, client_close2);
    assert!(result3.is_err());
    assert!(result4.is_err());
    assert!(result5.is_err());
  }

}
