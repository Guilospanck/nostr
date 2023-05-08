use std::{net::SocketAddr, sync::MutexGuard, vec};

use serde::{Deserialize, Serialize, Serializer, Deserializer, ser::SerializeSeq};

use crate::{
  event::Event,
  filter::Filter,
  relay::{ClientConnectionInfo, ClientRequests, Tx},
  relay_to_client_communication::event::RelayToClientCommEvent,
};

use super::{check_event_match_filter, Error};

#[derive(Debug, Clone)]
pub struct ClientToRelayCommRequest {
  pub code: String, // "REQ"
  pub subscription_id: String,
  pub filters: Vec<Filter>,
}

impl ClientToRelayCommRequest {
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
    Self::try_from(data).unwrap()
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

    if data_len == 0 || data_len == 1 {
      return Ok(Self {
        ..Default::default()
      });
    }

    if data_len == 2 {
      return Ok(Self {
        subscription_id: data[1].clone(),
        ..Default::default()
      });
    }

    let subscription_id = data[1].clone();
    let mut filters: Vec<Filter> = vec![];

    for filter in data[2..].iter() {
      match Filter::from_str(filter.clone()) {
        Ok(filter) => filters.push(filter),
        Err(e) => return Err(Error::Json(e)),
      }
    }

    Ok(Self {
      subscription_id,
      filters,
      ..Default::default()
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
    Ok(Self::from_vec(vec))
  }
}

/// Updates an already connected client -
/// overwriting the filters if they have the same
/// `subscription_id` or adding the new ones to the array -
/// or create a new one with this request.
///
/// Returns the saved events that match the requested filters.
///
pub fn on_request_message(
  client_request: ClientToRelayCommRequest,
  clients: &mut MutexGuard<Vec<ClientConnectionInfo>>,
  addr: SocketAddr,
  tx: Tx,
  events: &MutexGuard<Vec<Event>>,
) -> Vec<RelayToClientCommEvent> {
  // we need to do this because on the first time a client connects, it will send a `REQUEST` message
  // and we won't have it in our `clients` array yet.
  match clients.iter_mut().find(|client| client.socket_addr == addr) {
    Some(client) => {
      // client already exists, so his info should be updated
      match client
        .requests
        .iter_mut()
        .position(|req| req.subscription_id == client_request.subscription_id)
      {
        Some(index) => client.requests[index].filters = client_request.filters.clone(), // overwrites filters
        None => client.requests.push(ClientRequests {
          // adds new one to the array of requests of this connected client
          subscription_id: client_request.subscription_id.clone(),
          filters: client_request.filters.clone(),
        }),
      };
    }
    None => clients.push(ClientConnectionInfo {
      // creates a new client connection
      tx: tx.clone(),
      socket_addr: addr,
      requests: vec![ClientRequests {
        subscription_id: client_request.subscription_id.clone(),
        filters: client_request.filters.clone(),
      }],
    }),
  };

  // Check all events from the database that match the requested filter
  let mut events_to_send_to_client_that_match_the_requested_filter: Vec<RelayToClientCommEvent> =
    vec![];

  for filter in client_request.filters.iter() {
    let mut events_added_for_this_filter: Vec<RelayToClientCommEvent> = vec![];
    for event in events.iter() {
      if check_event_match_filter(event.clone(), filter.clone()) {
        events_added_for_this_filter.push(RelayToClientCommEvent {
          subscription_id: client_request.subscription_id.clone(),
          event: event.clone(),
          ..Default::default()
        });
      }
    }

    // Put the newest events first
    events_added_for_this_filter
      .sort_by(|event1, event2| event2.event.created_at.cmp(&event1.event.created_at));

    // Check limit of the filter as the REQ message will only be called on the first time something is required.
    if let Some(limit) = filter.limit {
      // Get up to the limit of the filter
      let slice = &events_added_for_this_filter.clone()[..limit as usize];
      events_added_for_this_filter = slice.to_vec();
    }
    events_to_send_to_client_that_match_the_requested_filter.extend(events_added_for_this_filter);
  }

  events_to_send_to_client_that_match_the_requested_filter
}

#[cfg(test)]
mod tests {
  use std::{
    net::{IpAddr, Ipv4Addr},
    sync::{Arc, Mutex},
    vec,
  };

  use crate::{
    event::{id::EventId, Timestamp},
    filter::Filter,
  };

  use super::*;

  #[cfg(test)]
  use pretty_assertions::assert_eq;
  use tokio_tungstenite::tungstenite::Message;

  struct ReqSut {
    mock_client_request: ClientToRelayCommRequest,
    mock_clients: Arc<Mutex<Vec<ClientConnectionInfo>>>,
    mock_addr: SocketAddr,
    mock_tx: Tx,
    mock_events: Arc<Mutex<Vec<Event>>>,
    mock_event: Event,
    mock_relay_to_client_event: RelayToClientCommEvent,
  }

  impl ReqSut {
    fn new(filter_limit: Option<Timestamp>) -> Self {
      let mock_clients: Arc<Mutex<Vec<ClientConnectionInfo>>> =
        Arc::new(Mutex::new(Vec::<ClientConnectionInfo>::new()));

      let mock_filter_id = String::from("05b25af3-4250-4fbf-8ef5-97220858f9ab");

      let mock_client_request = ClientToRelayCommRequest {
        code: "REQ".to_string(),
        subscription_id: "mock_subscription_id".to_string(),
        filters: vec![Filter {
          ids: Some(vec![EventId(mock_filter_id.clone())]),
          authors: None,
          kinds: None,
          e: None,
          p: None,
          since: None,
          until: None,
          limit: filter_limit,
        }],
      };

      let mock_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
      let (mock_tx, _rx) = futures_channel::mpsc::unbounded::<Message>();

      let empty_events: Vec<Event> = vec![];
      let mock_events = Arc::new(Mutex::new(empty_events));

      let mock_event = Self::mock_event(mock_filter_id);

      let mock_relay_to_client_event = RelayToClientCommEvent {
        subscription_id: mock_client_request.subscription_id.clone(),
        event: mock_event.clone(),
        ..Default::default()
      };

      Self {
        mock_addr,
        mock_client_request,
        mock_clients,
        mock_events,
        mock_tx,
        mock_event,
        mock_relay_to_client_event,
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
  fn test_on_req_msg_creates_new_client_request_and_returns_empty_array() {
    let mock = ReqSut::new(None);
    let mut clients = mock.mock_clients.lock().unwrap();
    let events = mock.mock_events.lock().unwrap();

    let events_to_send_to_client_that_match_the_requested_filter = on_request_message(
      mock.mock_client_request,
      &mut clients,
      mock.mock_addr,
      mock.mock_tx,
      &events,
    );

    assert_eq!(
      events_to_send_to_client_that_match_the_requested_filter,
      vec![]
    );
    assert_eq!(clients.len(), 1);
    assert_eq!(clients[0].socket_addr, mock.mock_addr);
  }

  #[test]
  fn test_on_req_msg_updates_existing_client_and_add_new_request_to_its_array_and_returns_empty_array(
  ) {
    let mock = ReqSut::new(None);
    let mut clients = mock.mock_clients.lock().unwrap();
    let events = mock.mock_events.lock().unwrap();
    clients.push(ClientConnectionInfo {
      tx: mock.mock_tx.clone(),
      socket_addr: mock.mock_addr,
      requests: vec![],
    });

    let events_to_send_to_client_that_match_the_requested_filter = on_request_message(
      mock.mock_client_request.clone(),
      &mut clients,
      mock.mock_addr,
      mock.mock_tx,
      &events,
    );

    assert_eq!(
      events_to_send_to_client_that_match_the_requested_filter,
      vec![]
    );
    assert_eq!(clients.len(), 1);
    assert_eq!(clients[0].socket_addr, mock.mock_addr);
    assert_eq!(clients[0].requests.len(), 1);
    assert_eq!(clients[0].requests.len(), 1);
    assert_eq!(
      clients[0].requests[0],
      ClientRequests {
        subscription_id: mock.mock_client_request.subscription_id,
        filters: mock.mock_client_request.filters
      }
    );
  }

  #[test]
  fn test_on_req_msg_updates_existing_client_and_also_its_request_array_and_returns_empty_array() {
    let mock = ReqSut::new(None);
    let mut clients = mock.mock_clients.lock().unwrap();
    let events = mock.mock_events.lock().unwrap();
    clients.push(ClientConnectionInfo {
      tx: mock.mock_tx.clone(),
      socket_addr: mock.mock_addr,
      requests: vec![ClientRequests {
        subscription_id: mock.mock_client_request.subscription_id.clone(),
        filters: vec![Filter::default()],
      }],
    });

    let events_to_send_to_client_that_match_the_requested_filter = on_request_message(
      mock.mock_client_request.clone(),
      &mut clients,
      mock.mock_addr,
      mock.mock_tx,
      &events,
    );

    assert_eq!(
      events_to_send_to_client_that_match_the_requested_filter,
      vec![]
    );
    assert_eq!(clients.len(), 1);
    assert_eq!(clients[0].socket_addr, mock.mock_addr);
    assert_eq!(clients[0].requests.len(), 1);
    assert_eq!(clients[0].requests.len(), 1);
    assert_eq!(
      clients[0].requests[0],
      ClientRequests {
        subscription_id: mock.mock_client_request.subscription_id,
        filters: mock.mock_client_request.filters
      }
    );
  }

  #[test]
  fn test_on_req_msg_creates_new_client_request_and_returns_events_that_match() {
    let mock = ReqSut::new(None);
    let mut clients = mock.mock_clients.lock().unwrap();
    let mut events = mock.mock_events.lock().unwrap();
    events.push(mock.mock_event.clone());

    let events_to_send_to_client_that_match_the_requested_filter = on_request_message(
      mock.mock_client_request,
      &mut clients,
      mock.mock_addr,
      mock.mock_tx,
      &events,
    );

    assert_eq!(
      events_to_send_to_client_that_match_the_requested_filter.len(),
      1
    );
    assert_eq!(
      events_to_send_to_client_that_match_the_requested_filter,
      vec![mock.mock_relay_to_client_event]
    );
    assert_eq!(clients.len(), 1);
    assert_eq!(clients[0].socket_addr, mock.mock_addr);
  }

  #[test]
  fn test_on_req_msg_should_respect_filter_limit() {
    let mock = ReqSut::new(Some(3));
    let mut clients = mock.mock_clients.lock().unwrap();
    let mut events = mock.mock_events.lock().unwrap();
    events.push(mock.mock_event.clone());
    events.push(mock.mock_event.clone());
    events.push(mock.mock_event.clone());
    events.push(mock.mock_event.clone());

    let events_to_send_to_client_that_match_the_requested_filter = on_request_message(
      mock.mock_client_request,
      &mut clients,
      mock.mock_addr,
      mock.mock_tx,
      &events,
    );

    assert_eq!(
      events_to_send_to_client_that_match_the_requested_filter.len(),
      3
    );
    assert_eq!(
      events_to_send_to_client_that_match_the_requested_filter,
      vec![
        mock.mock_relay_to_client_event.clone(),
        mock.mock_relay_to_client_event.clone(),
        mock.mock_relay_to_client_event
      ]
    );
    assert_eq!(clients.len(), 1);
    assert_eq!(clients[0].socket_addr, mock.mock_addr);
  }
}
