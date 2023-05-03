use std::{
  env,
  io::Error as IoError,
  net::SocketAddr,
  sync::{Arc, Mutex},
};

use futures_channel::mpsc::{unbounded, UnboundedSender};
use futures_util::{future, pin_mut, stream::TryStreamExt, StreamExt};

use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::Message;

use crate::{
  client_to_relay_communication::{
    on_close_message, on_event_message, on_request_message,
    types::{ClientToRelayCommClose, ClientToRelayCommEvent, ClientToRelayCommRequest},
  },
  db::EventsDB,
  event::Event,
  filter::Filter,
  relay_to_client_communication::{broadcast_message_to_clients, send_message_to_client},
};

pub type Tx = UnboundedSender<Message>;

/// Holds information about the requests made by a client.
///
#[derive(Debug, PartialEq, Eq)]
pub struct ClientRequests {
  pub subscription_id: String,
  pub filters: Vec<Filter>,
}

/// Holds information about the clients connection.
/// A client cannot have more than one connection with the same relay.
///
#[derive(Debug)]
pub struct ClientConnectionInfo {
  pub tx: Tx,
  pub socket_addr: SocketAddr,
  pub requests: Vec<ClientRequests>,
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
      on_close_message(msg_parsed.clone().data.close, &mut clients, addr);
    }

    if msg_parsed.is_request {
      let events_to_send_to_client = on_request_message(
        msg_parsed.clone().data.request,
        &mut clients,
        addr,
        tx.clone(),
        &events,
      );

      // Send to client all events matched
      let events_stringfied = serde_json::to_string(&events_to_send_to_client).unwrap();
      send_message_to_client(tx.clone(), events_stringfied);
    }

    if msg_parsed.is_event {
      let event = msg_parsed.data.event.event.clone();
      let event_stringfied = event.as_str();

      let mut mutable_events_db = events_db.lock().unwrap();

      // update the events array if this event doesn't already exist
      if !events.iter().any(|evt| evt.id == event.id) {
        events.push(event.clone());
        mutable_events_db
          .write_to_db((events.len() as u64) - 1, &event_stringfied)
          .unwrap();
      }

      let outbound_client_and_message = on_event_message(event, event_stringfied, &mut clients);

      // We want to broadcast the message to everyone that matches the filter.
      broadcast_message_to_clients(outbound_client_and_message);
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
