use std::sync::MutexGuard;

use crate::{
  event::Event, relay::ClientConnectionInfo, relay_to_client_communication::OutboundInfo,
};

use super::check_event_match_filter;

pub fn on_event_message(
  event: Event,
  event_stringfied: String,
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
            content: event_stringfied.clone(),
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
    client_to_relay_communication::types::ClientToRelayCommRequest,
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
  }

  impl EvtSut {
    fn new() -> Self {
      let mock_clients: Arc<Mutex<Vec<ClientConnectionInfo>>> =
        Arc::new(Mutex::new(Vec::<ClientConnectionInfo>::new()));

      let mock_filter_id = String::from("05b25af3-4250-4fbf-8ef5-97220858f9ab");

      let mock_filter = Filter {
        ids: Some(vec![EventId(mock_filter_id.clone())]),
        authors: None,
        kinds: None,
        e: None,
        p: None,
        since: None,
        until: None,
        limit: None,
      };

      let mock_client_request = ClientToRelayCommRequest {
        code: "REQ".to_string(),
        subscription_id: "mock_subscription_id".to_string(),
        filters: vec![mock_filter.clone()],
      };

      let mock_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
      let (mock_tx, _rx) = futures_channel::mpsc::unbounded::<Message>();

      let mock_event = Self::mock_event(mock_filter_id);

      Self {
        mock_addr,
        mock_client_request,
        mock_clients,
        mock_tx,
        mock_event,
        mock_filter,
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

    let outbound_client_and_message = on_event_message(
      mock.mock_event.clone(),
      mock.mock_event.as_str(),
      &mut clients,
    );

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

    let outbound_client_and_message = on_event_message(
      mock.mock_event.clone(),
      mock.mock_event.as_str(),
      &mut clients,
    );

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

    let outbound_client_and_message = on_event_message(
      mock.mock_event.clone(),
      mock.mock_event.as_str(),
      &mut clients,
    );

    assert_eq!(outbound_client_and_message.len(), 1);
  }
}
