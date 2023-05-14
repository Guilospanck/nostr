use std::vec;

use serde::{de, ser::SerializeSeq, Deserialize, Deserializer, Serialize, Serializer};

use crate::filter::Filter;

use super::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientToRelayCommRequest {
  pub code: String, // "REQ"
  pub subscription_id: String,
  pub filters: Vec<Filter>,
}

impl ClientToRelayCommRequest {
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

impl Default for ClientToRelayCommRequest {
  fn default() -> Self {
    Self {
      code: String::from("REQ"),
      subscription_id: String::new(),
      filters: vec![],
    }
  }
}

impl From<ClientToRelayCommRequest> for Vec<String> {
  fn from(data: ClientToRelayCommRequest) -> Self {
    let mut vec = vec![data.code, data.subscription_id];
    for filter in data.filters {
      vec.push(filter.as_str());
    }

    vec
  }
}

impl<S> TryFrom<Vec<S>> for ClientToRelayCommRequest
where
  S: Into<String>,
{
  type Error = Error;

  fn try_from(data: Vec<S>) -> Result<Self, Self::Error> {
    let data: Vec<String> = data.into_iter().map(|v| v.into()).collect();
    let data_len: usize = data.len();

    if data_len < 3 || data[0] != *"REQ" {
      return Err(Error::InvalidData);
    }

    let subscription_id = data[1].clone();
    let mut filters: Vec<Filter> = vec![];

    for filter in data[2..].iter() {
      match Filter::from_string(filter.clone()) {
        Ok(filter) => filters.push(filter),
        Err(e) => return Err(Error::Json(e)),
      }
    }

    Ok(Self {
      code: data[0].clone(),
      subscription_id,
      filters,
    })
  }
}

impl Serialize for ClientToRelayCommRequest {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    // using the `impl From<ClientToRelayCommRequest> for Vec<String>`
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

impl<'de> Deserialize<'de> for ClientToRelayCommRequest {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    type Data = Vec<String>;
    // Deserializes a string (serialized) into
    // a Vec<String>
    let vec: Vec<String> = Data::deserialize(deserializer)?;
    // Then it uses the `impl<S> From<Vec<S>> for ClientToRelayCommRequest` to retrieve the `ClientToRelayCommRequest` struct
    let result = Self::from_vec(vec);
    if result.is_err() {
      return Err(Error::InvalidData).map_err(de::Error::custom);
    }
    Ok(result.unwrap())
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
  use serde_json::json;

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
  fn test_client_to_relay_comm_request_as_str() {
    let mock = ReqSut::new(None);

    let mut client_request_for_expectation_2 = mock.mock_client_request.clone();
    client_request_for_expectation_2
      .filters
      .push(mock.mock_filter.clone());
    client_request_for_expectation_2
      .filters
      .push(mock.mock_filter.clone());

    let filter_as_str = mock.mock_filter.as_str();

    let expected = format!(r#"["REQ","mock_subscription_id","{}"]"#, filter_as_str);
    let expected2 = format!(
      r#"["REQ","mock_subscription_id","{}","{}","{}"]"#,
      filter_as_str, filter_as_str, filter_as_str
    );

    assert_eq!(
      expected,
      mock
        .mock_client_request
        .as_str()
        .unwrap()
        .replace("\\\"", "\"")
    );
    assert_eq!(
      expected2,
      client_request_for_expectation_2
        .as_str()
        .unwrap()
        .replace("\\\"", "\"")
    );
  }

  #[test]
  fn test_client_to_relay_comm_request_from_str() {
    let mock = ReqSut::new(None);

    let mut client_request_for_expectation_2 = mock.mock_client_request.clone();
    client_request_for_expectation_2
      .filters
      .push(mock.mock_filter.clone());
    client_request_for_expectation_2
      .filters
      .push(mock.mock_filter.clone());

    // let from_str = "[\"REQ\",\"mock_subscription_id\",\"{\\\"ids\\\":[\\\"05b25af3-4250-4fbf-8ef5-97220858f9ab\\\"],\\\"authors\\\":null,\\\"kinds\\\":null,\\\"#e\\\":null,\\\"#p\\\":null,\\\"since\\\":null,\\\"until\\\":null,\\\"limit\\\":null}\"]".to_owned();
    // let from_str2 = "[\"REQ\",\"mock_subscription_id\",\"{\\\"ids\\\":[\\\"05b25af3-4250-4fbf-8ef5-97220858f9ab\\\"],\\\"authors\\\":null,\\\"kinds\\\":null,\\\"#e\\\":null,\\\"#p\\\":null,\\\"since\\\":null,\\\"until\\\":null,\\\"limit\\\":null}\",\"{\\\"ids\\\":[\\\"05b25af3-4250-4fbf-8ef5-97220858f9ab\\\"],\\\"authors\\\":null,\\\"kinds\\\":null,\\\"#e\\\":null,\\\"#p\\\":null,\\\"since\\\":null,\\\"until\\\":null,\\\"limit\\\":null}\",\"{\\\"ids\\\":[\\\"05b25af3-4250-4fbf-8ef5-97220858f9ab\\\"],\\\"authors\\\":null,\\\"kinds\\\":null,\\\"#e\\\":null,\\\"#p\\\":null,\\\"since\\\":null,\\\"until\\\":null,\\\"limit\\\":null}\"]".to_owned();
    // let from_str3 = "[\"REQ\",\"9433794702187832\",\"{\\\"#e\\\":[\\\"44b17a5acd66694cbdf5aea08968453658446368d978a15e61e599b8404d82c4\\\",\\\"7742783afbf6b283e81af63782ab0c05bbcbccba7f3abce0e0f23706dc27bd42\\\",\\\"9621051bcd8723f03da00aae61ee46956936726fcdfa6f34e29ae8f1e2b63cb5\\\"],\\\"kinds\\\":[1,6,7,9735]}\"]".to_owned();

    let filter = json!({
      "ids":["05b25af3-4250-4fbf-8ef5-97220858f9ab"],"authors":null,"kinds":null,"#e":null,"#p":null,"since":null,"until":null,"limit":null
    }).to_string();
    let from_str = json!(["REQ", "mock_subscription_id", filter]).to_string();

    let from_str2 = json!(["REQ", "mock_subscription_id", filter, filter, filter]).to_string();

    let filter = json!({
      "kinds":[1,6,7,9735],
      "#e":["44b17a5acd66694cbdf5aea08968453658446368d978a15e61e599b8404d82c4","7742783afbf6b283e81af63782ab0c05bbcbccba7f3abce0e0f23706dc27bd42","9621051bcd8723f03da00aae61ee46956936726fcdfa6f34e29ae8f1e2b63cb5"]
    }).to_string();
    let from_str3 = json!(["REQ", "9433794702187832", filter]).to_string();

    let result = ClientToRelayCommRequest::from_string(from_str).unwrap();
    let result2 = ClientToRelayCommRequest::from_string(from_str2).unwrap();
    let result3 = ClientToRelayCommRequest::from_string(from_str3).unwrap();

    let expected_client_request_for_from_str_3 = ClientToRelayCommRequest {
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
    assert_eq!(result3, expected_client_request_for_from_str_3);
  }

  #[test]
  fn test_client_to_relay_comm_request_from_vec() {
    let mock = ReqSut::new(None);

    let mut client_request_for_expectation_2 = mock.mock_client_request.clone();
    client_request_for_expectation_2
      .filters
      .push(mock.mock_filter.clone());
    client_request_for_expectation_2
      .filters
      .push(mock.mock_filter.clone());

    let expected: Vec<String> = vec![
      "REQ".to_owned(),
      "mock_subscription_id".to_owned(),
      mock.mock_filter.as_str(),
    ];
    let expected2: Vec<String> = vec![
      "REQ".to_owned(),
      "mock_subscription_id".to_owned(),
      mock.mock_filter.as_str(),
      mock.mock_filter.as_str(),
      mock.mock_filter.as_str(),
    ];

    let result = ClientToRelayCommRequest::from_vec(expected).unwrap();
    let result2 = ClientToRelayCommRequest::from_vec(expected2).unwrap();

    assert_eq!(result, mock.mock_client_request);
    assert_eq!(result2, client_request_for_expectation_2);
  }

  #[test]
  fn test_client_to_relay_comm_request_as_vec() {
    let mock = ReqSut::new(None);

    let mut client_request_for_expectation_2 = mock.mock_client_request.clone();
    client_request_for_expectation_2
      .filters
      .push(mock.mock_filter.clone());
    client_request_for_expectation_2
      .filters
      .push(mock.mock_filter.clone());

    let expected: Vec<String> = vec![
      "REQ".to_owned(),
      "mock_subscription_id".to_owned(),
      mock.mock_filter.as_str(),
    ];
    let expected2: Vec<String> = vec![
      "REQ".to_owned(),
      "mock_subscription_id".to_owned(),
      mock.mock_filter.as_str(),
      mock.mock_filter.as_str(),
      mock.mock_filter.as_str(),
    ];

    assert_eq!(expected, mock.mock_client_request.as_vec());
    assert_eq!(expected2, client_request_for_expectation_2.as_vec());
  }
}
