use std::{
  env,
  io::Error as IoError,
  net::SocketAddr,
  sync::{Arc, Mutex, MutexGuard},
};

use futures_channel::mpsc::{unbounded, UnboundedSender};
use futures_util::{future, pin_mut, stream::TryStreamExt, StreamExt};

use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::protocol::Message;

use crate::{
  client_to_relay_comm::{
    ClientToRelayCommClose, ClientToRelayCommEvent, ClientToRelayCommRequest,
  },
  db::EventsDB,
  event::Event,
  filter::Filter,
};

type Tx = UnboundedSender<Message>;

/// Holds information about the requests made by a client.
///
#[derive(Debug)]
struct ClientRequests {
  subscription_id: String,
  filters: Vec<Filter>,
}

/// Holds information about the clients connection.
/// A client cannot have more than one connection with the same relay.
///
#[derive(Debug)]
struct ClientConnectionInfo {
  tx: Tx,
  socket_addr: SocketAddr,
  requests: Vec<ClientRequests>,
}

#[derive(Default, Clone)]
struct AnyCommunicationFromClient {
  close: ClientToRelayCommClose,
  event: ClientToRelayCommEvent,
  request: ClientToRelayCommRequest,
}

#[derive(Default, Clone)]
struct MsgResult {
  no_op: bool,
  is_close: bool,
  is_event: bool,
  is_request: bool,
  data: AnyCommunicationFromClient,
}

#[derive(Debug)]
struct OutboundInfo {
  tx: Tx,
  content: String,
}

fn check_filter_match_event(event: Event, filter: Filter) -> bool {
  let mut is_match = true;

  // Check IDs
  if let Some(ids) = filter.ids {
    let id_in_list = ids
      .iter()
      .any(|id| *id == event.id || id.starts_with(&event.id));
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

  // Check Tags
  if let Some(tags) = filter.tags {
    for tag in tags // { "e": ["event_id1", "event_id2", "..."], "p": ["pubkey_1", "pubkey_2", "..."] }
      .iter()
    {
      // tag = ( "e", ["event_id1", "event_id2", "..."] )

      // Check if event has the same tag as the filter is requesting
      let does_event_have_tag = event
        .tags
        .iter()
        .position(|event_tag| *tag.0 == event_tag[0]);

      match does_event_have_tag {
        Some(index) => {
          let is_event_id_referenced_in_requested_filter_tag = tag
            .1
            .iter()
            .any(|event_id| *event_id == event.tags[index][1]);

          if !is_event_id_referenced_in_requested_filter_tag {
            is_match = false;
          }
        }
        None => is_match = false, // if a filter requires a tag and the event doesn't have it, it's not a match
      }
    }
  }

  is_match
}

fn send_message_to_client(tx: Tx, content: String) {
  tx.unbounded_send(Message::binary(format!("{}", content).as_bytes()))
    .unwrap();
}

fn broadcast_message_to_clients(outbound_client_and_message: Vec<OutboundInfo>) {
  for recp in outbound_client_and_message {
    send_message_to_client(recp.tx.clone(), recp.content.clone());
  }
}

fn on_close_message(
  msg_parsed: MsgResult,
  clients: &mut MutexGuard<Vec<ClientConnectionInfo>>,
  addr: SocketAddr,
) {
  match clients.iter().position(|client| client.socket_addr == addr) {
    Some(client_idx) => {
      // Client can only close the subscription of its own connection
      match clients[client_idx]
        .requests
        .iter()
        .position(|client_req| client_req.subscription_id == msg_parsed.data.close.subscription_id)
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

fn on_request_message(
  msg_parsed: MsgResult,
  clients: &mut MutexGuard<Vec<ClientConnectionInfo>>,
  addr: SocketAddr,
  tx: UnboundedSender<Message>,
  events: &MutexGuard<Vec<Event>>,
) {
  // we need to do this because on the first time a client connects, it will send a `REQUEST` message
  // and we won't have it in our `clients` array yet.
  match clients.iter_mut().find(|client| client.socket_addr == addr) {
    Some(client) => {
      // client already exists, so his info should be updated
      match client
        .requests
        .iter_mut()
        .position(|req| req.subscription_id == msg_parsed.data.request.subscription_id)
      {
        Some(index) => client.requests[index].filters = msg_parsed.data.request.filters.clone(), // overwrites filters
        None => client.requests.push(ClientRequests {
          // adds new one to the array of requests of this connected client
          subscription_id: msg_parsed.data.request.subscription_id.clone(),
          filters: msg_parsed.data.request.filters.clone(),
        }),
      };
    }
    None => clients.push(ClientConnectionInfo {
      // creates a new client connection
      tx: tx.clone(),
      socket_addr: addr,
      requests: vec![ClientRequests {
        subscription_id: msg_parsed.data.request.subscription_id.clone(),
        filters: msg_parsed.data.request.filters.clone(),
      }],
    }),
  };

  // Check all events from the database that match the requested filter
  let mut events_to_send_to_client_that_match_the_requested_filter: Vec<Event> = vec![];

  events.iter().for_each(|event| {
    msg_parsed.data.request.filters.iter().for_each(|filter| {
      if check_filter_match_event(event.clone(), filter.clone()) {
        events_to_send_to_client_that_match_the_requested_filter.push(event.clone());
      }
    })
  });

  // Send to client all events matched
  let events_stringfied =
    serde_json::to_string(&events_to_send_to_client_that_match_the_requested_filter).unwrap();
  send_message_to_client(tx, events_stringfied);
}

fn on_event_message(
  event: Event,
  event_stringfied: String,
  clients: &mut MutexGuard<Vec<ClientConnectionInfo>>
) {
  let mut outbound_client_and_message: Vec<OutboundInfo> = vec![];

  // when an `event` message is received, it's because we are already connected to the client and, therefore,
  // we have its data stored in `clients`, so NO need to verify if he exists
  for client in clients.iter_mut() {
    // Check filters
    client.requests.iter().for_each(|client_req| {
      client_req.filters.iter().for_each(|filter| {
        if check_filter_match_event(event.clone(), filter.clone()) {
          outbound_client_and_message.push(OutboundInfo {
            tx: client.tx.clone(),
            content: event_stringfied.clone(),
          });
        }
      });
    });
  }

  // We want to broadcast the message to everyone that matches the filter.
  broadcast_message_to_clients(outbound_client_and_message);
}

/*
  Expects a message like:
  let msg = "[\"EVENT\",{\"id\":\"ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb\",\"pubkey\":\"02c7e1b1e9c175ab2d100baf1d5a66e73ecc044e9f8093d0c965741f26aa3abf76\",\"created_at\":1673002822,\"kind\":1,\"tags\":[[\"e\",\"688787d8ff144c502c7f5cffaafe2cc588d86079f9de88304c26b0cb99ce91c6\",\"wss://relay.damus.io\"],[\"p\",\"02c7e1b1e9c175ab2d100baf1d5a66e73ecc044e9f8093d0c965741f26aa3abf76\",\"\"]],\"content\":\"Lorem ipsum dolor sit amet\",\"sig\":\"e8551d85f530113366e8da481354c2756605e3f58149cedc1fb9385d35251712b954af8ef891cb0467d50ddc6685063d4190c97e9e131f903e6e4176dc13ce7c\"}]".to_owned();
  let msg = "[\"REQ\",\"asdf\",[\"{\"ids\":[\"ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb\"],\"authors\":null,\"kinds\":null,\"tags\":null,\"since\":null,\"until\":null,\"limit\":null}\"]]".to_owned();
  let msg = "[\"CLOSE\",\"asdf\"]".to_owned();
*/
fn parse_message_received_from_client(msg: &str) -> MsgResult {
  let mut result = MsgResult::default();

  if let Ok(close_msg) = serde_json::from_str::<ClientToRelayCommClose>(msg) {
    println!("Close:\n {:?}\n\n", close_msg);

    result.is_close = true;
    result.data.close = close_msg;
    return result;
  }

  if let Ok(event_msg) = serde_json::from_str::<ClientToRelayCommEvent>(msg) {
    println!("Event:\n {:?}\n\n", event_msg);

    result.is_event = true;
    result.data.event = event_msg.clone();
    return result;
  }

  if let Ok(request_msg) = serde_json::from_str::<ClientToRelayCommRequest>(msg) {
    println!("Request:\n {:?}\n\n", request_msg);

    result.is_request = true;
    result.data.request = request_msg;
    return result;
  }

  result.no_op = true;
  result
}

/// This function is called when the connection relay-client is closed.
fn connection_cleanup(
  client_connection_info: Arc<Mutex<Vec<ClientConnectionInfo>>>,
  addr: SocketAddr,
) {
  println!("Client with address {} disconnected", &addr);
  client_connection_info
    .lock()
    .unwrap()
    .retain(|client| client.socket_addr != addr);
}

async fn handle_connection(
  raw_stream: TcpStream,
  addr: SocketAddr,
  client_connection_info: Arc<Mutex<Vec<ClientConnectionInfo>>>,
  events: Arc<Mutex<Vec<Event>>>,
  events_db: Arc<Mutex<EventsDB<'_>>>,
) {
  println!("Incoming TCP connection from: {}", addr);

  let ws_stream = tokio_tungstenite::accept_async(raw_stream)
    .await
    .expect("Error during the websocket handshake occurred");
  println!("WebSocket connection established: {}", addr);

  let (tx, rx) = unbounded();

  let (outgoing, incoming) = ws_stream.split();

  let broadcast_incoming = incoming.try_for_each(|msg| {
    println!(
      "Received a message from {}: {}",
      addr,
      msg.to_text().unwrap()
    );

    let mut clients = client_connection_info.lock().unwrap();
    let mut events = events.lock().unwrap();

    let msg_parsed = parse_message_received_from_client(msg.to_text().unwrap());

    if msg_parsed.no_op {
      return future::ok(());
    }

    if msg_parsed.is_close {
      on_close_message(msg_parsed.clone(), &mut clients, addr);
    }

    if msg_parsed.is_request {
      on_request_message(msg_parsed.clone(), &mut clients, addr, tx.clone(), &events);
    }

    if msg_parsed.is_event {
      let event = msg_parsed.data.event.event.clone();
      let event_stringfied = serde_json::to_string(&event).unwrap();

      let mut mutable_events_db = events_db.lock().unwrap();

      // update the events array if this event doesn't already exist
      if !events.iter().any(|evt| evt.id == event.id) {
        events.push(event.clone());
        mutable_events_db
          .write_to_db((events.len() as u64) - 1, &event_stringfied)
          .unwrap();
      }

      on_event_message(event, event_stringfied, &mut clients);
    }

    future::ok(())
  });

  let receive_from_others = rx.map(Ok).forward(outgoing);

  pin_mut!(broadcast_incoming, receive_from_others);
  future::select(broadcast_incoming, receive_from_others).await;

  // If the code reaches this part it is because some of the futures above
  // (namely `broadcast_incoming` or `receive_from_others`) is done (connection is closed for some reason)
  // Therefore we need to do this cleanup
  connection_cleanup(client_connection_info, addr);
}

#[derive(Debug)]
pub enum MainError {
  IoError(IoError),
  RedbError(redb::Error),
}

#[tokio::main]
pub async fn initiate_relay() -> Result<(), MainError> {
  let addr = env::args()
    .nth(1)
    .unwrap_or_else(|| "127.0.0.1:8080".to_string());

  // Read events from DB
  let events_db = EventsDB::new().unwrap();
  let events = events_db.get_all_items().unwrap();

  // thread-safe and lockable
  let client_connection_info = Arc::new(Mutex::new(Vec::<ClientConnectionInfo>::new()));
  let events = Arc::new(Mutex::new(events));
  let events_db = Arc::new(Mutex::new(events_db));

  // Create the event loop and TCP listener we'll accept connections on.
  let try_socket = TcpListener::bind(&addr).await;
  let listener = try_socket.expect("Failed to bind");
  println!("Listening on: {}", addr);

  loop {
    // Asynchronously wait for an inbound TCPStream
    let (stream, addr) = listener.accept().await.unwrap();

    // Clone the states we want to be able to mutate
    // throughout different threads
    let client_connection_info = Arc::clone(&client_connection_info);
    let events = Arc::clone(&events);
    let events_db = Arc::clone(&events_db);

    // Spawn the handler to run async
    tokio::spawn(handle_connection(
      stream,
      addr,
      client_connection_info,
      events,
      events_db,
    ));
  }
}
