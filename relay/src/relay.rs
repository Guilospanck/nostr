use std::{
  env,
  io::Error as IoError,
  net::SocketAddr,
  sync::{Arc, Mutex},
};

use futures_channel::mpsc::UnboundedSender;
use futures_util::{future, pin_mut, stream::TryStreamExt, StreamExt};

use serde_json::json;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::Message;

use nostr_sdk::{
  client_to_relay_communication::{
    close::ClientToRelayCommClose, event::ClientToRelayCommEvent, request::ClientToRelayCommRequest,
  },
  event::Event,
  filter::Filter,
  relay_to_client_communication::{eose::RelayToClientCommEose, notice::RelayToClientCommNotice},
};

use crate::{
  db::EventsDB,
  receive_from_client::{
    close::on_close_message, event::on_event_message, request::on_request_message,
  },
  send_to_client::{broadcast_message_to_clients, send_message_to_client},
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

#[derive(Default, Clone, Debug)]
struct AnyCommunicationFromClient {
  close: ClientToRelayCommClose,
  event: ClientToRelayCommEvent,
  request: ClientToRelayCommRequest,
}

#[derive(Default, Debug, Clone)]
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
  let msg = "[\"REQ\",\"asdf\",
    \"{\"ids\":[\"ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb\"],\"authors\":null,\"kinds\":null,\"tags\":null,\"since\":null,\"until\":null,\"limit\":null}\",
    \"{\"ids\":[\"ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb\"],\"authors\":null,\"kinds\":null,\"tags\":null,\"since\":null,\"until\":null,\"limit\":null}\",...]".to_owned();
  let msg = "[\"CLOSE\",\"asdf\"]".to_owned();

  ["REQ","9433794702187832",{"#e":["44b17a5acd66694cbdf5aea08968453658446368d978a15e61e599b8404d82c4","7742783afbf6b283e81af63782ab0c05bbcbccba7f3abce0e0f23706dc27bd42","9621051bcd8723f03da00aae61ee46956936726fcdfa6f34e29ae8f1e2b63cb5"],"kinds":[1,6,7,9735]}]
*/
fn parse_message_received_from_client(msg: &str) -> MsgResult {
  let mut result = MsgResult::default();

  if let Ok(close_msg) = ClientToRelayCommClose::from_json(msg.to_string()) {
    println!("Close:\n {:?}\n\n", close_msg);

    result.is_close = true;
    result.data.close = close_msg;
    return result;
  }

  if let Ok(event_msg) = ClientToRelayCommEvent::from_json(msg.to_string()) {
    println!("Event:\n {:?}\n\n", event_msg);

    result.is_event = true;
    result.data.event = event_msg;
    return result;
  }

  if let Ok(request_msg) = ClientToRelayCommRequest::from_json(msg.to_string()) {
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
  let ws_stream = tokio_tungstenite::accept_async(raw_stream)
    .await
    .expect("Error during the websocket handshake occurred");
  println!("WebSocket connection established: {}", addr);

  let (tx, rx) = futures_channel::mpsc::unbounded();

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
      let closed = on_close_message(
        msg_parsed.clone().data.close.subscription_id,
        &mut clients,
        addr,
      );
      // Send NOTICE event to inform that the subscription was closed or not
      let message = if closed {
        "Subscription ended.".to_owned()
      } else {
        "Subscription not found.".to_owned()
      };
      let notice_event = RelayToClientCommNotice {
        message,
        ..Default::default()
      }
      .as_json();
      send_message_to_client(tx.clone(), notice_event);
    }

    if msg_parsed.is_request {
      let events_to_send_to_client = on_request_message(
        msg_parsed.clone().data.request.subscription_id,
        msg_parsed.clone().data.request.filters,
        &mut clients,
        addr,
        tx.clone(),
        &events,
      );

      // Send one event at a time
      for event_message in events_to_send_to_client {
        let events_stringfied = json!(event_message).to_string();
        send_message_to_client(tx.clone(), events_stringfied);
      }

      // Send EOSE event to indicate end of stored events
      let eose = RelayToClientCommEose {
        subscription_id: msg_parsed.clone().data.request.subscription_id,
        ..Default::default()
      };
      send_message_to_client(tx.clone(), eose.as_json());
    }

    if msg_parsed.is_event {
      let event = msg_parsed.data.event.event;
      let event_stringfied = event.as_json();

      let mut mutable_events_db = events_db.lock().unwrap();

      // update the events array if this event doesn't already exist
      if !events.iter().any(|evt| evt.id == event.id) {
        events.push(event.clone());
        mutable_events_db
          .write_to_db((events.len() as u64) - 1, &event_stringfied)
          .unwrap();
      }

      let outbound_client_and_message = on_event_message(event, &mut clients);

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
    .unwrap_or_else(|| "0.0.0.0:8080".to_string());

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

  // Handle CTRL+C signal
  let ctrl_c_listener = async {
    tokio::signal::ctrl_c().await.unwrap();
    let clients = client_connection_info.lock().unwrap();
    // close all open connections with clients
    async {
      for client in clients.iter() {
        let notice_event = RelayToClientCommNotice {
          message: format!("Server {addr} closing connection..."),
          ..Default::default()
        }
        .as_json();
        send_message_to_client(client.tx.clone(), notice_event);
        client.tx.close_channel();
      }
    }
    .await;
    println!("Ctrl-C received, shutting down");
  };

  // Spin up the server
  let server = async {
    while let Ok((stream, addr)) = listener.accept().await {
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
  };

  // Pinning the futures is necessary for using `select!`
  pin_mut!(server, ctrl_c_listener);
  // Whichever returns first, will end the server
  future::select(server, ctrl_c_listener).await;

  Ok(())
}
