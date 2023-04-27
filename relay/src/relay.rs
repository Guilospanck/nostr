use std::{
  collections::HashMap,
  env,
  fmt::format,
  io::Error as IoError,
  net::SocketAddr,
  sync::{Arc, Mutex, MutexGuard},
};

use futures_channel::mpsc::{unbounded, UnboundedSender};
use futures_util::{future, pin_mut, stream::TryStreamExt, StreamExt};

use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::protocol::Message;

use crate::{
  client_to_relay_comms::{
    ClientToRelayCommClose, ClientToRelayCommEvent, ClientToRelayCommRequest,
  },
  event::Event,
  filter::Filter,
};

type Tx = UnboundedSender<Message>;

struct ClientConnectionInfo {
  subscription_id: String,
  tx: Tx,
  socket_addr: SocketAddr,
  events: Vec<Event>,
  filter: Filter,
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

fn on_close_message(
  msg_parsed: MsgResult,
  clients: &mut MutexGuard<Vec<ClientConnectionInfo>>,
  addr: SocketAddr,
) -> bool {
  let client_idx_with_addr_and_subscription_id_exists = clients.iter().position(|client| {
    client.subscription_id == msg_parsed.data.close.subscription_id && client.socket_addr == addr
  });
  match client_idx_with_addr_and_subscription_id_exists {
    Some(client_index) => {
      clients.remove(client_index);
      return true;
    }
    None => return false,
  }
}

fn on_request_message(
  msg_parsed: MsgResult,
  clients: &mut MutexGuard<Vec<ClientConnectionInfo>>,
  addr: SocketAddr,
  tx: UnboundedSender<Message>,
) {
  // TODO: return to this peer all events in memory that match this filter
  //       (all conditions of the filter are treated as && conditions)

  // we need to do this because on the first time a client connects, it will send a `REQUEST` message
  // and we won't have it in our `clients` array yet.
  match clients.iter_mut().find(|client| client.socket_addr == addr) {
    Some(client) => client.filter = msg_parsed.data.request.filter, // update filter
    None => clients.push(ClientConnectionInfo {
      subscription_id: msg_parsed.data.request.subscription_id,
      tx: tx.clone(),
      socket_addr: addr,
      events: Vec::new(),
      filter: msg_parsed.data.request.filter,
    }),
  };
}

fn check_filter_match_event(event: Event, filter: Filter) -> bool {
  let mut is_match = true;

  // Check IDs
  if let Some(ids) = filter.ids {
    let id_in_list = ids
      .iter()
      .any(|id| *id == event.id || id.contains(&event.id));
    if !id_in_list {
      return false;
    }
  }

  // Check Authors
  if let Some(authors) = filter.authors {
    let author_in_list = authors
      .iter()
      .any(|author| *author == event.pubkey || author.contains(&event.pubkey));
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
          let does_event_id_is_referenced = tag
            .1
            .iter()
            .any(|event_id| *event_id == event.tags[index][1]);

          if !does_event_id_is_referenced {
            is_match = false;
          }
        }
        None => is_match = false, // if a filter requires a tag and the event doesn't have it, it's not a match
      }
    }
  }

  is_match
}

fn on_event_message(
  msg_parsed: MsgResult,
  clients: &mut MutexGuard<Vec<ClientConnectionInfo>>,
  addr: SocketAddr,
) {
  let mut message_to_send_to_client: Vec<OutboundInfo> = vec![];

  // when an event message is received, it's because we are already connected to the client and, therefore,
  // we have its data stored in `clients`, so no need to verify if he exists
  for client in clients.iter_mut() {
    let event: Event = msg_parsed.data.event.event.clone();
    let filter: Filter = client.filter.clone();

    if client.socket_addr == addr {
      client.events.push(event.clone());
    }

    // Check filter
    if check_filter_match_event(event.clone(), filter) {
      println!("NEW MATCHED FILTER => {:?}", client.socket_addr);
      message_to_send_to_client.push(OutboundInfo {
        tx: client.tx.clone(),
        content: event.content,
      });
    }
  }

  // We want to broadcast the message to everyone that matches the filter.
  for recp in message_to_send_to_client {
    recp
      .tx
      .unbounded_send(tokio_tungstenite::tungstenite::Message::Text(format!(
        "{}",
        recp.content,
      )))
      .unwrap();
  }
}

/*
  Expects a message like:
  let msg = "[\"EVENT\",{\"id\":\"ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb\",\"pubkey\":\"02c7e1b1e9c175ab2d100baf1d5a66e73ecc044e9f8093d0c965741f26aa3abf76\",\"created_at\":1673002822,\"kind\":1,\"tags\":[[\"e\",\"688787d8ff144c502c7f5cffaafe2cc588d86079f9de88304c26b0cb99ce91c6\",\"wss://relay.damus.io\"],[\"p\",\"02c7e1b1e9c175ab2d100baf1d5a66e73ecc044e9f8093d0c965741f26aa3abf76\",\"\"]],\"content\":\"Lorem ipsum dolor sit amet\",\"sig\":\"e8551d85f530113366e8da481354c2756605e3f58149cedc1fb9385d35251712b954af8ef891cb0467d50ddc6685063d4190c97e9e131f903e6e4176dc13ce7c\"}]".to_owned();
  let msg = "[\"REQ\",\"asdf\",\"{\"ids\":[\"ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb\"],\"authors\":null,\"kinds\":null,\"tags\":null,\"since\":null,\"until\":null,\"limit\":null}\"]".to_owned();
  let msg = "[\"CLOSE\",\"asdf\"]".to_owned();
*/
fn parse_message_received_from_client(msg: &str) -> MsgResult {
  let mut result = MsgResult::default();

  if let Ok(close_msg) = serde_json::from_str::<ClientToRelayCommClose>(msg) {
    println!("Close:\n {:?}", close_msg);

    result.is_close = true;
    result.data.close = close_msg;
    return result;
  }

  if let Ok(event_msg) = serde_json::from_str::<ClientToRelayCommEvent>(msg) {
    println!("Event:\n {:?}", event_msg);

    result.is_event = true;
    result.data.event = event_msg.clone();
    return result;
  }

  if let Ok(request_msg) = serde_json::from_str::<ClientToRelayCommRequest>(msg) {
    println!("Request:\n {:?}", request_msg);

    result.is_request = true;
    result.data.request = request_msg;
    return result;
  }

  result.no_op = true;
  result
}

async fn handle_connection(
  raw_stream: TcpStream,
  addr: SocketAddr,
  client_connection_info: Arc<Mutex<Vec<ClientConnectionInfo>>>,
) {
  // let start = SystemTime::now();
  // let since_epoch = start
  //   .duration_since(UNIX_EPOCH)
  //   .expect("Time went backwards");
  // println!("Time now in seconds: {}", since_epoch.as_secs());

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

    let msg_parsed = parse_message_received_from_client(msg.to_text().unwrap());

    if msg_parsed.no_op {
      return future::ok(());
    }

    if msg_parsed.is_close {
      return if on_close_message(msg_parsed, &mut clients, addr) == true {
        future::err(tokio_tungstenite::tungstenite::Error::ConnectionClosed)
      } else {
        future::ok(())
      };
    }

    if msg_parsed.is_request {
      on_request_message(msg_parsed.clone(), &mut clients, addr, tx.clone());
    }

    if msg_parsed.is_event {
      on_event_message(msg_parsed, &mut clients, addr);
    }

    future::ok(())
  });

  let receive_from_others = rx.map(Ok).forward(outgoing);

  pin_mut!(broadcast_incoming, receive_from_others);
  future::select(broadcast_incoming, receive_from_others).await;

  println!("{} disconnected", &addr);
  client_connection_info
    .lock()
    .unwrap()
    .retain(|client| client.socket_addr != addr);
}

#[tokio::main]
pub async fn initiate_relay() -> Result<(), IoError> {
  /*

  let ev = Event {
    id: "ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb".to_owned(),
    pubkey: "02c7e1b1e9c175ab2d100baf1d5a66e73ecc044e9f8093d0c965741f26aa3abf76".to_owned(),
    created_at: 1673002822,
    kind: 1,
    tags: Tags([
      ["e".to_owned(), "688787d8ff144c502c7f5cffaafe2cc588d86079f9de88304c26b0cb99ce91c6".to_owned(), "wss://relay.damus.io".to_owned()],
      ["p".to_owned(), "02c7e1b1e9c175ab2d100baf1d5a66e73ecc044e9f8093d0c965741f26aa3abf76".to_owned(), "".to_owned()],
    ].to_vec()),
    content: "Lorem ipsum dolor sit amet".to_owned(),
    sig: "e8551d85f530113366e8da481354c2756605e3f58149cedc1fb9385d35251712b954af8ef891cb0467d50ddc6685063d4190c97e9e131f903e6e4176dc13ce7c".to_owned()
  };

  println!("{}", serde_json::to_string(&ev).unwrap());

  // Serde JSON serialized event:

  // {"id":"ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb","pubkey":"02c7e1b1e9c175ab2d100baf1d5a66e73ecc044e9f8093d0c965741f26aa3abf76","created_at":1673002822,"kind":1,"tags":[["e","688787d8ff144c502c7f5cffaafe2cc588d86079f9de88304c26b0cb99ce91c6","wss://relay.damus.io"],["p","02c7e1b1e9c175ab2d100baf1d5a66e73ecc044e9f8093d0c965741f26aa3abf76",""]],"content":"Lorem ipsum dolor sit amet","sig":"e8551d85f530113366e8da481354c2756605e3f58149cedc1fb9385d35251712b954af8ef891cb0467d50ddc6685063d4190c97e9e131f903e6e4176dc13ce7c"}

  let event_test = get_event_id(ev);


  // Serialized test event:

  // [0,"02c7e1b1e9c175ab2d100baf1d5a66e73ecc044e9f8093d0c965741f26aa3abf76",1673002822,1,[["e","688787d8ff144c502c7f5cffaafe2cc588d86079f9de88304c26b0cb99ce91c6","wss://relay.damus.io"],["p","02c7e1b1e9c175ab2d100baf1d5a66e73ecc044e9f8093d0c965741f26aa3abf76",""]],"Lorem ipsum dolor sit amet"]

  */

  let addr = env::args()
    .nth(1)
    .unwrap_or_else(|| "127.0.0.1:8080".to_string());

  // thread-safe and lockable
  let client_connection_info = Arc::new(Mutex::new(Vec::<ClientConnectionInfo>::new()));

  // Create the event loop and TCP listener we'll accept connections on.
  let try_socket = TcpListener::bind(&addr).await;
  let listener = try_socket.expect("Failed to bind");
  println!("Listening on: {}", addr);

  loop {
    // Asynchronously wait for an inbound TCPStream
    let (stream, addr) = listener.accept().await?;

    // Clone the states we want to be able to mutate
    // throughout different threads
    let client_connection_info = Arc::clone(&client_connection_info);

    // Spawn the handler to run async
    tokio::spawn(handle_connection(stream, addr, client_connection_info));
  }
}
