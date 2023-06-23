use std::vec;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{json, Value};

use crate::filter::Filter;

use super::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientToRelayCommRequest {
  pub code: String, // "REQ"
  pub subscription_id: String,
  pub filters: Vec<Filter>,
}

impl ClientToRelayCommRequest {
  /// Create new `REQ` message
  pub fn new_req(subscription_id: String, filters: Vec<Filter>) -> Self {
    Self {
      code: "REQ".to_string(),
      subscription_id,
      filters,
    }
  }

  /// Get request as JSON string
  pub fn as_json(&self) -> String {
    self.as_value().to_string()
  }

  /// Deserialize [`ClientToRelayCommRequest`] from JSON string
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
    let mut json = json!(["REQ", self.subscription_id]);
    let mut filters = json!(self.filters);

    if let Some(json) = json.as_array_mut() {
      if let Some(filters) = filters.as_array_mut() {
        json.append(filters);
      }
    }

    json
  }

  /// Deserialize from [`Value`]
  pub fn from_value(msg: Value) -> Result<Self, Error> {
    let v = msg.as_array().ok_or(Error::InvalidData)?;

    if v.is_empty() {
      return Err(Error::InvalidData);
    }

    let v_len: usize = v.len();

    // Req
    // ["REQ", <subscription_id>, <filter JSON>, <filter JSON>...]
    if v[0] == "REQ" {
      // A client can choose to only connect to a relay, without 
      // querying any data
      if v_len == 2 {
        let subscription_id = serde_json::from_value(v[1].clone())?;
        return Ok(Self::new_req(subscription_id, Vec::new()));
      } else if v_len >= 3 {
        let subscription_id = serde_json::from_value(v[1].clone())?;
        let filters: Vec<Filter> = serde_json::from_value(Value::Array(v[2..].to_vec()))?;
        return Ok(Self::new_req(subscription_id, filters));
      }
    }

    Err(Error::InvalidData)
  }
}

impl Default for ClientToRelayCommRequest {
  fn default() -> Self {
    Self {
      code: String::from("REQ"),
      subscription_id: String::new(),
      filters: vec![],
    }
  }
}

impl Serialize for ClientToRelayCommRequest {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    let json_value: Value = self.as_value();
    json_value.serialize(serializer)
  }
}

impl<'de> Deserialize<'de> for ClientToRelayCommRequest {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    // We don't know what we're receiving. So just try to deserialize it
    // to some value
    let json_value = Value::deserialize(deserializer)?;

    // If the deserialization happens correctly (i.e.: is a valid JSON),
    // We verify if this JSON is the one we want, namely `ClientToRelayCommRequest`
    ClientToRelayCommRequest::from_value(json_value).map_err(serde::de::Error::custom)
  }
}

#[cfg(test)]
mod tests {
  use crate::{
    event::{id::EventId, kind::EventKind, Timestamp},
    filter::Filter,
  };

  use super::*;

  #[cfg(test)]
  use pretty_assertions::assert_eq;

  struct ReqSut {
    mock_client_request: ClientToRelayCommRequest,
    mock_filter: Filter,
  }

  impl ReqSut {
    fn new(filter_limit: Option<Timestamp>) -> Self {
      let mock_filter_id = String::from("05b25af3-4250-4fbf-8ef5-97220858f9ab");

      let mock_filter: Filter = Filter {
        ids: Some(vec![EventId(mock_filter_id)]),
        authors: None,
        kinds: None,
        e: None,
        p: None,
        since: None,
        until: None,
        limit: filter_limit,
      };

      let mock_client_request = ClientToRelayCommRequest {
        code: "REQ".to_string(),
        subscription_id: "mock_subscription_id".to_string(),
        filters: vec![mock_filter.clone()],
      };

      Self {
        mock_client_request,
        mock_filter,
      }
    }
  }

  #[test]
  fn test_client_to_relay_comm_request_default() {
    let expected = ClientToRelayCommRequest {
      code: "REQ".to_owned(),
      subscription_id: "".to_owned(),
      filters: vec![],
    };

    let result = ClientToRelayCommRequest::default();

    assert_eq!(expected, result);
  }

  #[test]
  fn test_client_to_relay_comm_request_as_json() {
    let mock = ReqSut::new(None);

    let mut client_request_for_expectation_2 = mock.mock_client_request.clone();
    client_request_for_expectation_2
      .filters
      .push(mock.mock_filter.clone());
    client_request_for_expectation_2
      .filters
      .push(mock.mock_filter.clone());

    let filter_as_str = mock.mock_filter.as_str();

    let expected = ClientToRelayCommRequest::from_json(format!(r#"["REQ","mock_subscription_id",{}]"#, filter_as_str)).unwrap();
    let expected2 = ClientToRelayCommRequest::from_json(format!(
      r#"["REQ","mock_subscription_id",{},{},{}]"#,
      filter_as_str, filter_as_str, filter_as_str
    )).unwrap();

    let result = ClientToRelayCommRequest::from_json(mock.mock_client_request.as_json()).unwrap();
    let result2 = ClientToRelayCommRequest::from_json(client_request_for_expectation_2.as_json()).unwrap();

    assert_eq!(expected, result);
    assert_eq!(expected2, result2);
  }

  #[test]
  fn test_client_to_relay_comm_request_from_json() {
    let mock = ReqSut::new(None);

    let mut client_request_for_expectation_2 = mock.mock_client_request.clone();
    client_request_for_expectation_2
      .filters
      .push(mock.mock_filter.clone());
    client_request_for_expectation_2
      .filters
      .push(mock.mock_filter.clone());

    let filter = json!({
      "ids":["05b25af3-4250-4fbf-8ef5-97220858f9ab"],"authors":null,"kinds":null,"#e":null,"#p":null,"since":null,"until":null,"limit":null
    });
    let from_json = json!(["REQ", "mock_subscription_id", filter]).to_string();

    let from_json2 = json!(["REQ", "mock_subscription_id", filter, filter, filter]).to_string();

    let filter = json!({
      "kinds":[1,6,7,9735],
      "#e":["44b17a5acd66694cbdf5aea08968453658446368d978a15e61e599b8404d82c4","7742783afbf6b283e81af63782ab0c05bbcbccba7f3abce0e0f23706dc27bd42","9621051bcd8723f03da00aae61ee46956936726fcdfa6f34e29ae8f1e2b63cb5"]
    });
    let from_json3 = json!(["REQ", "9433794702187832", filter]).to_string();

    let result = ClientToRelayCommRequest::from_json(from_json).unwrap();
    let result2 = ClientToRelayCommRequest::from_json(from_json2).unwrap();
    let result3 = ClientToRelayCommRequest::from_json(from_json3).unwrap();

    let expected_client_request_for_from_json_3 = ClientToRelayCommRequest {
      code: "REQ".to_string(),
      subscription_id: "9433794702187832".to_string(),
      filters: vec![Filter {
        e: Some(vec![
          "44b17a5acd66694cbdf5aea08968453658446368d978a15e61e599b8404d82c4".to_string(),
          "7742783afbf6b283e81af63782ab0c05bbcbccba7f3abce0e0f23706dc27bd42".to_string(),
          "9621051bcd8723f03da00aae61ee46956936726fcdfa6f34e29ae8f1e2b63cb5".to_string(),
        ]),
        kinds: Some(vec![
          EventKind::Text,
          EventKind::Custom(6),
          EventKind::Custom(7),
          EventKind::Custom(9735),
        ]),
        ..Default::default()
      }],
    };

    assert_eq!(result, mock.mock_client_request);
    assert_eq!(result2, client_request_for_expectation_2);
    assert_eq!(result3, expected_client_request_for_from_json_3);
  }
}
