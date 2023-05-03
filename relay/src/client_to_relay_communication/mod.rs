use std::{net::SocketAddr, sync::MutexGuard};

use crate::{
  event::{
    tag::{Tag, TagKind},
    Event,
  },
  filter::Filter,
  relay::{ClientConnectionInfo, ClientRequests, Tx},
  relay_to_client_communication::OutboundInfo,
};

use self::types::{ClientToRelayCommClose, ClientToRelayCommRequest};

// Internal `client_to_relay_communication` modules
pub mod types;

fn check_event_match_filter(event: Event, filter: Filter) -> bool {
  // Check IDs
  if let Some(ids) = filter.ids {
    let id_in_list = ids
      .iter()
      .any(|id| *id.0 == event.id || id.0.starts_with(&event.id));
    if !id_in_list {
      return false;
    }
  }

  // Check Authors
  if let Some(authors) = filter.authors {
    let author_in_list = authors
      .iter()
      .any(|author| *author == event.pubkey || author.starts_with(&event.pubkey));
    if !author_in_list {
      return false;
    }
  }

  // Check Kinds
  if let Some(kinds) = filter.kinds {
    let kind_in_list = kinds.iter().any(|kind| *kind == event.kind);
    if !kind_in_list {
      return false;
    }
  }

  // Check Since
  if let Some(since) = filter.since {
    let event_after_since = since <= event.created_at;
    if !event_after_since {
      return false;
    }
  }

  // Check Until
  if let Some(until) = filter.until {
    let event_before_until = until >= event.created_at;
    if !event_before_until {
      return false;
    }
  }

  // Check #e tag
  if let Some(event_ids) = filter.e {
    match event
      .tags
      .iter()
      .position(|event_tag| TagKind::from(event_tag.clone()) == TagKind::Event)
    {
      Some(index) => {
        if let Tag::Event(event_event_tag_id, _, _) = &event.tags[index] {
          if !event_ids
            .iter()
            .any(|event_id| *event_id == event_event_tag_id.0)
          {
            return false;
          }
        }
      }
      None => return false,
    }
  }

  // Check #p tag
  if let Some(pubkeys) = filter.p {
    match event
      .tags
      .iter()
      .position(|event_tag| TagKind::from(event_tag.clone()) == TagKind::PubKey)
    {
      Some(index) => {
        if let Tag::PubKey(event_pubkey_tag_pubkey, _) = &event.tags[index] {
          if !pubkeys
            .iter()
            .any(|pubkey| *pubkey == *event_pubkey_tag_pubkey)
          {
            return false;
          }
        }
      }
      None => return false,
    }
  }

  true
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
) -> Vec<Event> {
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
  let mut events_to_send_to_client_that_match_the_requested_filter: Vec<Event> = vec![];

  for filter in client_request.filters.iter() {
    let mut events_added_for_this_filter: Vec<Event> = vec![];
    for event in events.iter() {
      if check_event_match_filter(event.clone(), filter.clone()) {
        events_added_for_this_filter.push(event.clone());
      }
    }
    // Check limit of the filter as the REQ message will only be called on the first time something is required.
    if let Some(limit) = filter.limit {
      // Put the newest events first
      events_added_for_this_filter
        .sort_by(|event1, event2| event2.created_at.cmp(&event1.created_at));
      // Get up to the limit of the filter
      let slice = &events_added_for_this_filter.clone()[..limit as usize];
      events_added_for_this_filter = slice.to_vec();
    }
    events_to_send_to_client_that_match_the_requested_filter.extend(events_added_for_this_filter);
  }

  events_to_send_to_client_that_match_the_requested_filter
}

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

pub fn on_close_message(
  client_close: ClientToRelayCommClose,
  clients: &mut MutexGuard<Vec<ClientConnectionInfo>>,
  addr: SocketAddr,
) {
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
        }
        None => (),
      }
    }
    None => (),
  };
}

#[cfg(test)]
mod tests {
  use std::{
    clone,
    net::{IpAddr, Ipv4Addr},
    sync::{Arc, Mutex},
    vec,
  };

  use crate::event::{id::EventId, Timestamp};

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
    mock_event: Event
  }

  impl ReqSut {
    fn new(filter_limit: Option<Timestamp>) -> Self {
      let mock_clients: Arc<Mutex<Vec<ClientConnectionInfo>>> = Arc::new(Mutex::new(Vec::<ClientConnectionInfo>::new()));

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

      Self {
        mock_addr,
        mock_client_request,
        mock_clients: mock_clients,
        mock_events,
        mock_tx,
        mock_event
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
      vec![mock.mock_event]
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
      vec![mock.mock_event.clone(), mock.mock_event.clone(), mock.mock_event]
    );
    assert_eq!(clients.len(), 1);
    assert_eq!(clients[0].socket_addr, mock.mock_addr);
  }
}
