use std::{
  env,
  io::Error as IoError,
  net::SocketAddr,
  sync::{Arc, Mutex},
};

use futures_channel::mpsc::UnboundedSender;
use futures_util::{future, pin_mut, stream::TryStreamExt, FutureExt, StreamExt};

use log::{debug, error, info};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{self, Duration};
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

/// Helper to parse the function into CLOSE, REQ or EVENT.
///
fn parse_message_received_from_client(msg: &str) -> MsgResult {
  let mut result = MsgResult::default();

  if let Ok(close_msg) = ClientToRelayCommClose::from_json(msg.to_string()) {
    debug!("Close:\n {:?}\n\n", close_msg);

    result.is_close = true;
    result.data.close = close_msg;
    return result;
  }

  if let Ok(event_msg) = ClientToRelayCommEvent::from_json(msg.to_string()) {
    debug!("Event:\n {:?}\n\n", event_msg);

    result.is_event = true;
    result.data.event = event_msg;
    return result;
  }

  if let Ok(request_msg) = ClientToRelayCommRequest::from_json(msg.to_string()) {
    debug!("Request:\n {:?}\n\n", request_msg);

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
  info!("Client with address {} disconnected", &addr);
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
  info!("WebSocket connection established: {addr}");

  // Start a periodic timer to send ping messages
  let ping_interval = Duration::from_secs(20);
  let mut interval = time::interval(ping_interval);

  let (tx, rx) = futures_channel::mpsc::unbounded();

  let (outgoing, incoming) = ws_stream.split();

  // Spawn the handler to run async
  let tx_clone = tx.clone();
  let ping = async {
    loop {
      interval.tick().await;

      // Send a ping message
      let ping_message = Message::Ping(vec![]);
      if let Err(err) = tx_clone.unbounded_send(ping_message) {
        error!("Error sending ping message: {err}");
        break Err(err).map_err(|_err| {
          tokio_tungstenite::tungstenite::Error::Protocol(
            tokio_tungstenite::tungstenite::error::ProtocolError::SendAfterClosing,
          )
        });
      }
      debug!("Sent ping to {addr}.");
    }
  };

  let broadcast_incoming = incoming.try_for_each(|msg| {
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
      // Send NOTICE event to inform if the subscription was closed or not
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
        send_message_to_client(tx.clone(), event_message.as_json());
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

      // verify event signature and event id. If it is not valid,
      // doesn't transmit it
      if !event.check_event_signature() || !event.check_event_id() {
        return future::ok(());
      }

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

  // This has to be done in order to:
  // - pin the future in the heap (Box::pin)
  // - be able to compose the vec in `select_all` (all will have the same "Box" type)
  let boxed_broadcast_incoming = broadcast_incoming.boxed();
  let receive_from_others = receive_from_others.boxed();
  let ping = ping.boxed();

  let (_, _, _) =
    future::select_all(vec![boxed_broadcast_incoming, receive_from_others, ping]).await;

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
  let addr = env::var("RELAY_HOST").unwrap_or_else(|_| "0.0.0.0:8080".to_string());

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
  info!("Listening on: {addr}");

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
        client.tx.unbounded_send(Message::Close(None)).unwrap();
      }
    }
    .await;
    info!("Ctrl-C received, shutting down");
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