pub mod communication_with_client;
pub mod database;
pub mod pool;
pub mod receive_from_client;
pub mod send_to_client;

use std::{
  env,
  io::Error as IoError,
  net::SocketAddr,
  sync::{Arc, Mutex},
};

use futures_util::{future, pin_mut, stream::TryStreamExt, FutureExt, SinkExt, StreamExt};

use log::{debug, error, info};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{self, Duration};
use tokio_tungstenite::tungstenite::Message;

use crate::{
  client::communication_with_relay::{
    close::ClientToRelayCommClose, event::ClientToRelayCommEvent, request::ClientToRelayCommRequest,
  },
  event::Event,
  filter::Filter,
  relay::{
    communication_with_client::{eose::RelayToClientCommEose, notice::RelayToClientCommNotice},
    database::EventsDB,
  },
};

use crate::relay::{
  receive_from_client::{
    close::on_close_message, event::on_event_message, request::on_request_message,
  },
  send_to_client::{broadcast_message_to_clients, send_message_to_client},
};

pub type Tx = tokio::sync::mpsc::UnboundedSender<Message>;

/// Holds information about the requests made by a client.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientRequests {
  pub subscription_id: String,
  pub filters: Vec<Filter>,
}

/// Holds information about the clients connection.
/// A client cannot have more than one connection with the same relay.
///
#[derive(Debug, Clone)]
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

  let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

  let (mut outgoing, incoming) = ws_stream.split();

  // Spawn the handler to run async
  let tx_clone = tx.clone();
  let ping = async {
    loop {
      interval.tick().await;

      // Send a ping message
      let ping_message = Message::Ping(vec![]);
      if let Err(err) = tx_clone.send(ping_message) {
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

  let rx_to_client = async {
    let mut result: Result<(), tokio_tungstenite::tungstenite::Error> = Ok(());

    while let Some(msg) = rx.recv().await {
      if let Err(err) = outgoing.send(msg.clone()).await {
        error!("Error sending {}: {err}", msg.to_string());
        result = Err(err).map_err(|_err| {
          tokio_tungstenite::tungstenite::Error::Protocol(
            tokio_tungstenite::tungstenite::error::ProtocolError::SendAfterClosing,
          )
        });
        break;
      }
    }

    result
  };

  // This has to be done in order to:
  // - pin the future in the heap (Box::pin)
  // - be able to compose the vec in `select_all` (all will have the same "Box" type)
  let boxed_broadcast_incoming = broadcast_incoming.boxed();
  let ping = ping.boxed();
  let rx_to_client = rx_to_client.boxed();

  let (_, _, _) = future::select_all(vec![boxed_broadcast_incoming, ping, rx_to_client]).await;

  // If the code reaches this part it is because some of the futures above
  // (namely `broadcast_incoming` or `ping` or `rx_to_client`) is done (connection is closed for some reason).
  // Therefore we need to do this cleanup.
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
        client.tx.send(Message::Close(None)).unwrap();
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

#[cfg(test)]
mod tests {
  use std::net::{IpAddr, Ipv4Addr};

  use super::*;

  #[cfg(test)]
  use pretty_assertions::assert_eq;
  use serde_json::json;

  fn make_clientconnectioninfo_sut(socket_addr: SocketAddr) -> ClientConnectionInfo {
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<Message>();

    ClientConnectionInfo {
      tx,
      socket_addr,
      requests: vec![],
    }
  }

  #[test]
  fn parse_close_message() {
    let close = ClientToRelayCommClose::default();
    let close_json = close.as_json();

    let result = parse_message_received_from_client(&close_json);

    assert_eq!(result.data.close, close);
    assert!(result.is_close);
    assert_eq!(result.is_event, false);
    assert_eq!(result.is_request, false);
    assert_eq!(result.no_op, false);
  }

  #[test]
  fn parse_request_message() {
    let request = ClientToRelayCommRequest::default();
    let request_json = request.as_json();

    let result = parse_message_received_from_client(&request_json);

    assert_eq!(result.data.request, request);
    assert!(result.is_request);
    assert_eq!(result.is_event, false);
    assert_eq!(result.is_close, false);
    assert_eq!(result.no_op, false);
  }

  #[test]
  fn parse_event_message() {
    let event_with_correct_signature = Event::from_value(
      json!({"content":"potato","created_at":1684589418,"id":"00960bd35499f8c63a4f65e79d6b1a2b7f1b8c97e76652325567b78c496350ae","kind":1,"pubkey":"614a695bab54e8dc98946abdb8ec019599ece6dada0c23890977d0fa128081d6","sig":"bf073c935f71de50ec72bdb79f75b0bf32f9049305c3b22f97c06422c6f2edc86e0d7e07d7d7222678b238b1daee071be5f6fa653c611971395ec0d1c6407caf","tags":[]}),
    ).unwrap();
    let event = ClientToRelayCommEvent::new_event(event_with_correct_signature);
    let event_json = event.as_json();

    let result = parse_message_received_from_client(&event_json);

    assert_eq!(result.data.event, event);
    assert!(result.is_event);
    assert_eq!(result.is_request, false);
    assert_eq!(result.is_close, false);
    assert_eq!(result.no_op, false);
  }

  #[test]
  fn parse_noop_message() {
    let no_op = r#"{}"#;

    let result = parse_message_received_from_client(no_op);

    assert!(result.no_op);
    assert_eq!(result.is_request, false);
    assert_eq!(result.is_close, false);
    assert_eq!(result.is_event, false);
  }

  #[test]
  fn test_connection_cleanup() {
    let client_connection_info = Arc::new(Mutex::new(Vec::<ClientConnectionInfo>::new()));
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
    let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8081);

    let client1 = make_clientconnectioninfo_sut(addr);
    let client2 = make_clientconnectioninfo_sut(addr2);

    // add some clients prior to cleanup
    let mut clients = client_connection_info.lock().unwrap();
    clients.push(client1);
    clients.push(client2.clone());
    assert_eq!(clients.len(), 2);
    drop(clients);

    connection_cleanup(client_connection_info.clone(), addr);

    let clients = client_connection_info.lock().unwrap();
    assert_eq!(clients.len(), 1);
    assert_eq!(clients.first().unwrap().requests, client2.requests);
    assert_eq!(clients.first().unwrap().socket_addr, client2.socket_addr);
  }
}
