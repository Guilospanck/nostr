use std::{net::SocketAddr, sync::MutexGuard, vec};

use crate::{
  client::communication_with_relay::check_event_match_filter, event::Event, filter::Filter,
  relay::communication_with_client::event::RelayToClientCommEvent,
};

use crate::relay::{ClientConnectionInfo, ClientRequests, Tx};

/// Updates an already connected client -
/// overwriting the filters if they have the same
/// `subscription_id` or adding the new ones to the array -
/// or create a new one with this request.
///
/// Returns the saved events that match the requested filters.
///
pub fn on_request_message(
  subscription_id: String,
  filters: Vec<Filter>,
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
        .position(|req| req.subscription_id == subscription_id)
      {
        Some(index) => client.requests[index].filters = filters.clone(), // overwrites filters
        None => client.requests.push(ClientRequests {
          // adds new one to the array of requests of this connected client
          subscription_id: subscription_id.clone(),
          filters: filters.clone(),
        }),
      };
    }
    None => clients.push(ClientConnectionInfo {
      // creates a new client connection
      tx,
      socket_addr: addr,
      requests: vec![ClientRequests {
        subscription_id: subscription_id.clone(),
        filters: filters.clone(),
      }],
    }),
  };

  // Check all events from the database that match the requested filter
  let mut events_to_send_to_client_that_match_the_requested_filter: Vec<RelayToClientCommEvent> =
    vec![];

  for filter in filters.iter() {
    let mut events_added_for_this_filter: Vec<RelayToClientCommEvent> = vec![];
    for event in events.iter() {
      if check_event_match_filter(event.clone(), filter.clone()) {
        events_added_for_this_filter.push(RelayToClientCommEvent {
          subscription_id: subscription_id.clone(),
          event: event.clone(),
          ..Default::default()
        });
      }
    }

    // Put the newest events first
    events_added_for_this_filter
      .sort_by(|event1, event2| event2.event.created_at.cmp(&event1.event.created_at));

    let events_added_length = events_added_for_this_filter.len();
    if events_added_length != 0 {
      // Check limit of the filter as the REQ message will only be called on the first time something is required.
      if let Some(limit) = filter.limit {
        let limit: usize = if (limit as usize) < events_added_length {
          limit as usize
        } else {
          events_added_length - 1
        };
        // Get up to the limit defined by the filter
        let slice = &events_added_for_this_filter.clone()[..limit];
        events_added_for_this_filter = slice.to_vec();
      }
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
    mock_clients: Arc<Mutex<Vec<ClientConnectionInfo>>>,
    mock_addr: SocketAddr,
    mock_tx: Tx,
    mock_events: Arc<Mutex<Vec<Event>>>,
    mock_event: Event,
    mock_relay_to_client_event: RelayToClientCommEvent,
    mock_filters: Vec<Filter>,
    mock_subscription_id: String,
  }

  impl ReqSut {
    fn new(filter_limit: Option<Timestamp>) -> Self {
      let mock_clients: Arc<Mutex<Vec<ClientConnectionInfo>>> =
        Arc::new(Mutex::new(Vec::<ClientConnectionInfo>::new()));

      let mock_filter_id = String::from("05b25af3-4250-4fbf-8ef5-97220858f9ab");

      let mock_filter: Filter = Filter {
        ids: Some(vec![EventId(mock_filter_id.clone())]),
        authors: None,
        kinds: None,
        e: None,
        p: None,
        since: None,
        until: None,
        limit: filter_limit,
      };

      let mock_subscription_id = String::from("potato");

      let mock_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
      let (mock_tx, _rx) = tokio::sync::mpsc::unbounded_channel::<Message>();

      let empty_events: Vec<Event> = vec![];
      let mock_events = Arc::new(Mutex::new(empty_events));

      let mock_event = Self::mock_event(mock_filter_id);

      let mock_relay_to_client_event = RelayToClientCommEvent {
        subscription_id: mock_subscription_id.clone(),
        event: mock_event.clone(),
        ..Default::default()
      };

      let mock_filters = vec![mock_filter];

      Self {
        mock_addr,
        mock_clients,
        mock_events,
        mock_tx,
        mock_event,
        mock_relay_to_client_event,
        mock_filters,
        mock_subscription_id,
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
      mock.mock_subscription_id,
      mock.mock_filters,
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
      mock.mock_subscription_id.clone(),
      mock.mock_filters.clone(),
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
        subscription_id: mock.mock_subscription_id,
        filters: mock.mock_filters
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
        subscription_id: mock.mock_subscription_id.clone(),
        filters: vec![Filter::default()],
      }],
    });

    let events_to_send_to_client_that_match_the_requested_filter = on_request_message(
      mock.mock_subscription_id.clone(),
      mock.mock_filters.clone(),
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
        subscription_id: mock.mock_subscription_id,
        filters: mock.mock_filters
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
      mock.mock_subscription_id,
      mock.mock_filters,
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
      mock.mock_subscription_id,
      mock.mock_filters,
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
