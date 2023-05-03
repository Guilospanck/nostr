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
