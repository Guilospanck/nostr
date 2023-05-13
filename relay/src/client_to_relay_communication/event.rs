use serde::{ser::SerializeSeq, Deserialize, Deserializer, Serialize, Serializer};
use std::sync::MutexGuard;

use crate::{
  event::Event,
  relay::ClientConnectionInfo,
  relay_to_client_communication::{event::RelayToClientCommEvent, OutboundInfo},
};

use super::check_event_match_filter;
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

  pub fn from_vec(data: Vec<String>) -> Self {
    Self::from(data)
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

impl<S> From<Vec<S>> for ClientToRelayCommEvent
where
  S: Into<String>,
{
  fn from(client_to_relay_event: Vec<S>) -> Self {
    let client_to_relay_event: Vec<String> = client_to_relay_event
      .into_iter()
      .map(|v| v.into())
      .collect();

    let length = client_to_relay_event.len();

    if length == 0 || length == 1 {
      return Self::default();
    }

    Self {
      event: Event::from_serialized(&client_to_relay_event[1].clone()),
      ..Default::default()
    }
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
    Ok(Self::from_vec(vec))
  }
}

pub fn on_event_message(
  event: Event,
  clients: &mut MutexGuard<Vec<ClientConnectionInfo>>,
) -> Vec<OutboundInfo> {
  let mut outbound_client_and_message: Vec<OutboundInfo> = vec![];

  // when an `event` message is received, it's because we are already connected to the client and, therefore,
  // we have its data stored in `clients`, so NO need to verify if he exists
  for client in clients.iter_mut() {
    // Check filters
    'outer: for client_req in client.requests.iter() {
      for filter in client_req.filters.iter() {
        if check_event_match_filter(event.clone(), filter.clone()) {
          outbound_client_and_message.push(OutboundInfo {
            tx: client.tx.clone(),
            content: RelayToClientCommEvent {
              subscription_id: client_req.subscription_id.clone(),
              event: event.clone(),
              ..Default::default()
            }
            .as_content(),
          });
          // I can break from going through client requests
          // because I have already found that this client requests
          // this event, therefore after adding him to the
          // `outbound_client_and_message` array, I can go
          // to the next one.
          break 'outer;
        }
      }
    }
  }

  outbound_client_and_message
}

#[cfg(test)]
mod tests {
  use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{Arc, Mutex},
  };

  use crate::{
    client_to_relay_communication::request::ClientToRelayCommRequest,
    event::id::EventId,
    filter::Filter,
    relay::{ClientRequests, Tx},
  };

  use super::*;

  #[cfg(test)]
  use pretty_assertions::assert_eq;
  use tokio_tungstenite::tungstenite::Message;

  struct EvtSut {
    mock_client_request: ClientToRelayCommRequest,
    mock_clients: Arc<Mutex<Vec<ClientConnectionInfo>>>,
    mock_addr: SocketAddr,
    mock_tx: Tx,
    mock_event: Event,
    mock_filter: Filter,
    mock_client_event: ClientToRelayCommEvent,
  }

  impl EvtSut {
    fn new() -> Self {
      let mock_clients: Arc<Mutex<Vec<ClientConnectionInfo>>> =
        Arc::new(Mutex::new(Vec::<ClientConnectionInfo>::new()));

      let mock_filter_id = String::from("05b25af3-4250-4fbf-8ef5-97220858f9ab");

      let mock_event_id = EventId(mock_filter_id.clone());

      let mock_filter = Filter {
        ids: Some(vec![mock_event_id]),
        authors: None,
        kinds: None,
        e: None,
        p: None,
        since: None,
        until: None,
        limit: None,
      };

      let mock_client_request = ClientToRelayCommRequest {
        code: "EVENT".to_string(),
        subscription_id: "mock_subscription_id".to_string(),
        filters: vec![mock_filter.clone()],
      };

      let mock_event = Self::mock_event(mock_filter_id);

      let mock_client_event = ClientToRelayCommEvent {
        code: "EVENT".to_string(),
        event: mock_event.clone(),
      };

      let mock_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
      let (mock_tx, _rx) = futures_channel::mpsc::unbounded::<Message>();

      Self {
        mock_addr,
        mock_client_request,
        mock_clients,
        mock_tx,
        mock_event,
        mock_filter,
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
  fn test_on_event_message_returns_empty_array_when_no_event_match() {
    let mock = EvtSut::new();
    let mut clients = mock.mock_clients.lock().unwrap();

    let outbound_client_and_message = on_event_message(mock.mock_event.clone(), &mut clients);

    assert_eq!(outbound_client_and_message.len(), 0);
  }

  #[test]
  fn test_on_event_message_returns_one_client_that_matches_filter() {
    let mock = EvtSut::new();
    let mut clients = mock.mock_clients.lock().unwrap();
    clients.push(ClientConnectionInfo {
      tx: mock.mock_tx.clone(),
      socket_addr: mock.mock_addr,
      requests: vec![ClientRequests {
        subscription_id: mock.mock_client_request.subscription_id.clone(),
        filters: mock.mock_client_request.filters,
      }],
    });

    let outbound_client_and_message = on_event_message(mock.mock_event.clone(), &mut clients);

    assert_eq!(outbound_client_and_message.len(), 1);
  }

  #[test]
  fn test_on_event_message_returns_one_client_that_matches_filter_even_with_more_than_one_filter() {
    let mock = EvtSut::new();
    let mut clients = mock.mock_clients.lock().unwrap();
    clients.push(ClientConnectionInfo {
      tx: mock.mock_tx.clone(),
      socket_addr: mock.mock_addr,
      requests: vec![ClientRequests {
        subscription_id: mock.mock_client_request.subscription_id.clone(),
        filters: vec![vec![mock.mock_filter], mock.mock_client_request.filters].concat(),
      }],
    });

    let outbound_client_and_message = on_event_message(mock.mock_event.clone(), &mut clients);

    assert_eq!(outbound_client_and_message.len(), 1);
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

    let expected: Vec<String> = vec![];
    let expected2: Vec<String> = vec!["EVENT".to_owned()];
    let expected3: Vec<String> = vec!["EVENT".to_owned(), mock.mock_event.as_str()];

    let result = ClientToRelayCommEvent::from_vec(expected);
    let result2 = ClientToRelayCommEvent::from_vec(expected2);
    let result3 = ClientToRelayCommEvent::from_vec(expected3);

    assert_eq!(result, ClientToRelayCommEvent::default());
    assert_eq!(result2, ClientToRelayCommEvent::default());
    assert_eq!(result3, mock.mock_client_event);
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
