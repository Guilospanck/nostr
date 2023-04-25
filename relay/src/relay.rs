use std::{
  collections::HashMap,
  env,
  io::Error as IoError,
  net::SocketAddr,
  sync::{Arc, Mutex, MutexGuard},
};

use futures_channel::mpsc::{unbounded, UnboundedSender};
use futures_util::{future, pin_mut, stream::TryStreamExt, StreamExt};

use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::protocol::Message;

use crate::client_to_relay_comms::{ClientToRelayCommEvent, ClientToRelayCommRequest, ClientToRelayCommClose};

type Tx = UnboundedSender<Message>;
type PeerMap = Arc<Mutex<HashMap<SocketAddr, Tx>>>;

struct MsgResult {
  is_close: bool,
  is_event: bool,
  is_filter: bool,
}

/*
  Expects a message like:
  let msg = "[\"EVENT\",{\"id\":\"ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb\",\"pubkey\":\"02c7e1b1e9c175ab2d100baf1d5a66e73ecc044e9f8093d0c965741f26aa3abf76\",\"created_at\":1673002822,\"kind\":1,\"tags\":[[\"e\",\"688787d8ff144c502c7f5cffaafe2cc588d86079f9de88304c26b0cb99ce91c6\",\"wss://relay.damus.io\"],[\"p\",\"02c7e1b1e9c175ab2d100baf1d5a66e73ecc044e9f8093d0c965741f26aa3abf76\",\"\"]],\"content\":\"Lorem ipsum dolor sit amet\",\"sig\":\"e8551d85f530113366e8da481354c2756605e3f58149cedc1fb9385d35251712b954af8ef891cb0467d50ddc6685063d4190c97e9e131f903e6e4176dc13ce7c\"}]".to_owned();
  let msg = "[\"REQ\",\"asdf\",\"{\"ids\":[\"ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb\"],\"authors\":null,\"kinds\":null,\"tags\":null,\"since\":null,\"until\":null,\"limit\":null}\"]".to_owned();
  let msg = "[\"CLOSE\",\"asdf\"]".to_owned();
*/
fn parse_msg_from_client(
  msg: &str,
  mutable_events: &mut MutexGuard<Vec<ClientToRelayCommEvent>>,
  mutable_filters: &mut MutexGuard<Vec<ClientToRelayCommRequest>>,
) -> MsgResult {
  let mut result = MsgResult {
    is_close: false,
    is_event: false,
    is_filter: false,
  };

  if serde_json::from_str::<ClientToRelayCommEvent>(msg).is_ok() {
    let data = serde_json::from_str::<ClientToRelayCommEvent>(msg).unwrap();
    println!("Event:\n {:?}", data);
    mutable_events.push(data.clone());
    result.is_event = true;
    return result;
  }

  if serde_json::from_str::<ClientToRelayCommRequest>(msg).is_ok() {
    let data = serde_json::from_str::<ClientToRelayCommRequest>(msg).unwrap();
    println!("Request:\n {:?}", data);
    mutable_filters.push(data.clone());
    result.is_filter = true;
    return result;
  }

  if serde_json::from_str::<ClientToRelayCommClose>(msg).is_ok() {
    result.is_close = true;
    return result;
  }

  result
}

async fn handle_connection(
  peer_map: PeerMap,
  raw_stream: TcpStream,
  addr: SocketAddr,
  events: Arc<Mutex<Vec<ClientToRelayCommEvent>>>,
  filters: Arc<Mutex<Vec<ClientToRelayCommRequest>>>,
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

  // Insert the write part of this peer to the peer map.
  let (tx, rx) = unbounded();
  peer_map.lock().unwrap().insert(addr, tx);

  let (outgoing, incoming) = ws_stream.split();

  let broadcast_incoming = incoming.try_for_each(|msg| {
    println!(
      "Received a message from {}: {}",
      addr,
      msg.to_text().unwrap()
    );

    let mut peers = peer_map.lock().unwrap();
    let mut mutable_events = events.lock().unwrap();
    let mut mutable_filters = filters.lock().unwrap();

    let msg_parsed = parse_msg_from_client(
      msg.to_text().unwrap(),
      &mut mutable_events,
      &mut mutable_filters,
    );

    if msg_parsed.is_close {
      // TODO: remove disconnected peer and filters/events from him
      peers.retain(|peer_addr, _| peer_addr != &addr);
      return future::err(tokio_tungstenite::tungstenite::Error::ConnectionClosed);
    }

    if msg_parsed.is_filter {
      // TODO: return to this peer all events in memory that match this filter.
    }

    if msg_parsed.is_event {
      // TODO: verify event against all saved filters and send it to matched ones
    }

    // We want to broadcast the message to everyone except ourselves.
    let broadcast_recipients = peers
      .iter()
      .filter(|(peer_addr, _)| peer_addr != &&addr)
      .map(|(_, ws_sink)| ws_sink);

    for recp in broadcast_recipients {
      recp
        .unbounded_send(tokio_tungstenite::tungstenite::Message::Text(format!(
          "Number of events: {}\nNumber of filters: {}\n\n",
          mutable_events.len(),
          mutable_filters.len(),
        )))
        .unwrap();
    }

    future::ok(())
  });

  let receive_from_others = rx.map(Ok).forward(outgoing);

  pin_mut!(broadcast_incoming, receive_from_others);
  future::select(broadcast_incoming, receive_from_others).await;

  println!("{} disconnected", &addr);
  peer_map.lock().unwrap().remove(&addr);
}

struct PeerMapAndSubscriptionID {
  peer: PeerMap,
  subscription_id: String,
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
  let events = Arc::new(Mutex::new(Vec::<ClientToRelayCommEvent>::new()));
  let filters = Arc::new(Mutex::new(Vec::<ClientToRelayCommRequest>::new()));
  let state = PeerMap::new(Mutex::new(HashMap::new()));

  // Create the event loop and TCP listener we'll accept connections on.
  let try_socket = TcpListener::bind(&addr).await;
  let listener = try_socket.expect("Failed to bind");
  println!("Listening on: {}", addr);

  loop {
    // Asynchronously wait for an inbound TCPStream
    let (stream, addr) = listener.accept().await?;

    // Clone the states we want to be able to mutate
    // throughout different threads
    let events = Arc::clone(&events);
    let filters = Arc::clone(&filters);
    let state = Arc::clone(&state);

    // Spawn the handler to run async
    println!("New connection attempt!!!\n\n\n");
    tokio::spawn(handle_connection(state, stream, addr, events, filters));
  }
}
