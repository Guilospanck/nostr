use serde::{ser::SerializeSeq, Deserialize, Deserializer, Serialize, Serializer};
use std::{net::SocketAddr, sync::MutexGuard};

use crate::relay::ClientConnectionInfo;

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

  pub fn from_str(data: String) -> Result<Self, Error> {
    serde_json::from_str(&data).map_err(Error::Json)
  }

  pub fn as_vec(&self) -> Vec<String> {
    self.clone().into()
  }

  pub fn from_vec(data: Vec<String>) -> Self {
    Self::from(data)
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

impl<S> From<Vec<S>> for ClientToRelayCommClose
where
  S: Into<String>,
{
  fn from(value: Vec<S>) -> Self {
    let value: Vec<String> = value.into_iter().map(|v| v.into()).collect();
    let length = value.len();

    if length == 0 || length == 1 {
      return Self::default();
    }

    Self {
      code: String::from("CLOSE"),
      subscription_id: value[1].clone(),
    }
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
    Ok(Self::from_vec(vec))
  }
}

pub fn on_close_message(
  client_close: ClientToRelayCommClose,
  clients: &mut MutexGuard<Vec<ClientConnectionInfo>>,
  addr: SocketAddr,
) -> bool {
  match clients.iter().position(|client| client.socket_addr == addr) {
    Some(client_idx) => {
      // Client can only close the subscription of its own connection
      match clients[client_idx]
        .requests
        .iter()
        .position(|client_req| client_req.subscription_id == client_close.subscription_id)
      {
        Some(client_req_index) => {
          clients[client_idx].requests.remove(client_req_index);
          true
        }
        None => false,
      }
    }
    None => false,
  }
}

#[cfg(test)]
mod tests {
  use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{Arc, Mutex},
  };

  use crate::{
    filter::Filter,
    relay::{ClientRequests, Tx},
  };

  use super::*;

  #[cfg(test)]
  use pretty_assertions::assert_eq;
  use tokio_tungstenite::tungstenite::Message;

  struct CloseSut {
    mock_client_close: ClientToRelayCommClose,
    mock_clients: Arc<Mutex<Vec<ClientConnectionInfo>>>,
    mock_addr: SocketAddr,
    mock_tx: Tx,
    mock_subscription_id: String,
  }

  impl CloseSut {
    fn new() -> Self {
      let mock_clients: Arc<Mutex<Vec<ClientConnectionInfo>>> =
        Arc::new(Mutex::new(Vec::<ClientConnectionInfo>::new()));

      let mock_subscription_id = "mock_subscription_id".to_string();

      let mock_client_close = ClientToRelayCommClose {
        code: "CLOSE".to_string(),
        subscription_id: mock_subscription_id.clone(),
      };

      let mock_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);

      let (mock_tx, _rx) = futures_channel::mpsc::unbounded::<Message>();

      Self {
        mock_addr,
        mock_client_close,
        mock_clients,
        mock_tx,
        mock_subscription_id,
      }
    }
  }

  #[test]
  fn test_on_event_message_should_do_nothing_when_socket_addresses_are_not_equal() {
    let mock = CloseSut::new();
    let mut clients = mock.mock_clients.lock().unwrap();
    clients.push(ClientConnectionInfo {
      tx: mock.mock_tx.clone(),
      socket_addr: mock.mock_addr,
      requests: vec![ClientRequests {
        subscription_id: mock.mock_subscription_id,
        filters: vec![Filter::default()],
      }],
    });
    let another_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8081);

    on_close_message(mock.mock_client_close, &mut clients, another_addr);

    assert_eq!(clients.len(), 1);
    assert_eq!(clients[0].requests.len(), 1);
  }

  #[test]
  fn test_on_event_message_should_do_nothing_when_subscription_ids_are_not_equal() {
    let mock = CloseSut::new();
    let mut clients = mock.mock_clients.lock().unwrap();
    clients.push(ClientConnectionInfo {
      tx: mock.mock_tx.clone(),
      socket_addr: mock.mock_addr,
      requests: vec![ClientRequests {
        subscription_id: "another_subs_id".to_string(),
        filters: vec![Filter::default()],
      }],
    });

    on_close_message(mock.mock_client_close, &mut clients, mock.mock_addr);

    assert_eq!(clients.len(), 1);
    assert_eq!(clients[0].requests.len(), 1);
  }

  #[test]
  fn test_on_event_message_should_remove_client_reqs() {
    let mock = CloseSut::new();
    let mut clients = mock.mock_clients.lock().unwrap();
    clients.push(ClientConnectionInfo {
      tx: mock.mock_tx.clone(),
      socket_addr: mock.mock_addr,
      requests: vec![ClientRequests {
        subscription_id: mock.mock_subscription_id,
        filters: vec![Filter::default()],
      }],
    });

    on_close_message(mock.mock_client_close, &mut clients, mock.mock_addr);

    assert_eq!(clients.len(), 1);
    assert_eq!(clients[0].requests.len(), 0);
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

    let result = ClientToRelayCommClose::from_str(expected).unwrap();
    let result2 = ClientToRelayCommClose::from_str(expected2).unwrap();
    let result3 = ClientToRelayCommClose::from_str(expected3).unwrap();
    let result4 = ClientToRelayCommClose::from_str(expected4).unwrap();
    let result5 = ClientToRelayCommClose::from_str(expected5).unwrap();

    let client_close2 = ClientToRelayCommClose::default();

    assert_eq!(result, mock.mock_client_close);
    assert_eq!(result2, client_close2);
    assert_eq!(result3, client_close2);
    assert_eq!(result4, client_close2);
    assert_eq!(result5, client_close2);
  }

  #[test]
  fn test_client_to_relay_comm_close_from_vec() {
    let mock = CloseSut::new();

    let expected: Vec<String> = vec!["CLOSE".to_owned(), "mock_subscription_id".to_owned()];
    let expected2: Vec<String> = vec!["CLOSE".to_owned(), "".to_owned()];
    let expected3: Vec<String> = vec!["CLOSE".to_owned()];
    let expected4: Vec<String> = vec!["".to_owned()];
    let expected5: Vec<String> = vec![];

    let result = ClientToRelayCommClose::from_vec(expected);
    let result2 = ClientToRelayCommClose::from_vec(expected2);
    let result3 = ClientToRelayCommClose::from_vec(expected3);
    let result4 = ClientToRelayCommClose::from_vec(expected4);
    let result5 = ClientToRelayCommClose::from_vec(expected5);

    let default_client_close = ClientToRelayCommClose::default();

    assert_eq!(result, mock.mock_client_close);
    assert_eq!(result2, default_client_close);
    assert_eq!(result3, default_client_close);
    assert_eq!(result4, default_client_close);
    assert_eq!(result5, default_client_close);
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
