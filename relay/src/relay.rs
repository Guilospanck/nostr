use std::{
  collections::HashMap,
  env,
  io::Error as IoError,
  net::SocketAddr,
  sync::{Arc, Mutex},
  thread, str::FromStr,
};

use bitcoin_hashes::{sha1, sha256, Hmac, Hash};
use futures_channel::mpsc::{unbounded, UnboundedSender};
use futures_util::{future, pin_mut, stream::TryStreamExt, StreamExt};

use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::protocol::{frame::coding::Data, Message};

use serde::{Deserialize, Serialize};

type Tx = UnboundedSender<Message>;
type PeerMap = Arc<Mutex<HashMap<SocketAddr, Tx>>>;

use std::time::{SystemTime, UNIX_EPOCH};
/**
* Nostr
*
* Client-to-Relay:
   * ["EVENT", <event JSON>] -> used to publish events
   * ["REQ", <subscription_id>, <filters JSON] -> used to request events and subscribe to new updates
   * ["CLOSE", <subscription_id>] -> used to stop previous subscriptions

   <subscription_id>: random string used to represent a subscription.

*/
use uuid::Uuid;

/**
 Filters are data structures that clients send to relays (being the first on the first connection)
 to request data from other clients.
 The attributes of a Filter work as && (in other words, all the conditions set must be present
 in the event in order to pass the filter)

 - ids: a list of events of prefixes
 - authors: a list of publickeys or prefixes, the pubkey of an event must be one of these
 - kinds: a list of kind numbers
 - tags: list of tags
   [
     e: a list of event ids that are referenced in an "e" tag,
     p: a list of pubkeys that are referenced in an "p" tag,
     ...
   ]
 - since: a timestamp. Events must be newer than this to pass
 - until: a timestamp. Events must be older than this to pass
 - limit: maximum number of events to be returned in the initial query
*/

#[derive(Debug, Serialize, Deserialize)]
pub struct Filter {
  ids: Option<Vec<String>>,
  authors: Option<Vec<String>>,
  kinds: Option<Vec<u64>>,
  tags: Option<HashMap<String, Vec<String>>>,
  since: Option<String>,
  until: Option<String>,
  limit: Option<u64>,
}

pub enum EventTags {
  PubKey,
  Event,
}

impl EventTags {
  fn as_str(&self) -> &'static str {
    match self {
      EventTags::PubKey => "p", // points to a pubkey of someone that is referred to in the event
      EventTags::Event => "e", // points to the id of an event this event is quoting, replying to or referring to somehow.
    }
  }
}

/** A tag is made of 3 parts:
   - an EventTag (p, e ...)
   - a string informing the content for that EventTag (pubkey for the "p" tag and event id for the "e" tag)
   - an optional string of a recommended relay URL (can be set to "")

   ```["p", <32-bytes hex of the key>], <recommended relay URL>]```
   ```["e", <32-bytes hex of the id of another event>, <recommended relay URL>]```

   Example:
   ```json
   ["e", "688787d8ff144c502c7f5cffaafe2cc588d86079f9de88304c26b0cb99ce91c6", "wss://relay.damus.io"]
   ["p", "02c7e1b1e9c175ab2d100baf1d5a66e73ecc044e9f8093d0c965741f26aa3abf76", ""]
   ```
*/
pub type Tag = [String; 3];

#[derive(Debug, Deserialize, Serialize)]
pub struct Tags(Vec<Tag>);

impl std::fmt::Display for Tags {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    write!(f, "[")?;
    for (idx, tag) in self.0.iter().enumerate() {
      write!(f, "[")?;
      for (pos, v) in tag.iter().enumerate() {
        write!(f, "\"{}\"", v)?;
        if pos < tag.len() - 1 {
          write!(f, ",")?;
        }
      }
      write!(f, "]")?;
      if idx < self.0.len() - 1 {
        write!(f, ",")?;
      }
    }
    write!(f, "]")?;
    Ok(())
  }
}

pub enum EventKinds {
  Metadata = 0,
  Text = 1,
  RecommendRelay = 2,
  Contacts = 3,
  EncryptedDirectMessages = 4,
  EventDeletion = 5,
  Repost = 6,
  Reaction = 7,
  ChannelCreation = 40,
  ChannelMetadata = 41,
  ChannelMessage = 42,
  ChannelHideMessage = 43,
  ChannelMuteUser = 44,
}

/**
 Event is the only object that exists in the Nostr protocol.

 Example (id's and other hashes are not valid for the information presented):
   ```json
   {
     "id": "ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb"
     "pubkey": "02c7e1b1e9c175ab2d100baf1d5a66e73ecc044e9f8093d0c965741f26aa3abf76",
     "created_at": 1673002822,
     "kind": 1,
     "tags": [
       ["e", "688787d8ff144c502c7f5cffaafe2cc588d86079f9de88304c26b0cb99ce91c6", "wss://relay.damus.io"],
       ["p", "02c7e1b1e9c175ab2d100baf1d5a66e73ecc044e9f8093d0c965741f26aa3abf76", ""],
     ],
     "content": "Lorem ipsum dolor sit amet",
     "sig": "e8551d85f530113366e8da481354c2756605e3f58149cedc1fb9385d35251712b954af8ef891cb0467d50ddc6685063d4190c97e9e131f903e6e4176dc13ce7c"
   }
   ```
*/
#[derive(Debug, Deserialize, Serialize)]
pub struct Event {
  id: String,      // 32-bytes SHA256 of the serialized event data
  pubkey: String,  // 32-bytes hex-encoded public key of the event creator
  created_at: u64, // unix timestamp in seconds
  kind: u64,       // kind of event
  tags: Tags,
  content: String, // arbitrary string
  sig: String,     // 64-bytes signature of the id field
}

/**
  This is the way used to serialize and get the SHA256. This will equal to `event.id`.
 */
fn get_event_id(event: Event) -> String {
  let data = format!(
    "[{},\"{}\",{},{},{},\"{}\"]",
    0, event.pubkey, event.created_at, event.kind, event.tags, event.content
  );

  let hash = sha256::Hash::hash(&data.as_bytes());
  hash.to_string()
}

async fn handle_connection(
  peer_map: PeerMap,
  raw_stream: TcpStream,
  addr: SocketAddr,
  events: Arc<Mutex<Vec<String>>>,
  filters: Arc<Mutex<Vec<String>>>,
) {
  let start = SystemTime::now();
  let since_epoch = start
    .duration_since(UNIX_EPOCH)
    .expect("Time went backwards");
  println!("Time now in seconds: {}", since_epoch.as_secs());

  println!("Incoming TCP connection from: {}", addr);

  let ws_stream = tokio_tungstenite::accept_async(raw_stream)
    .await
    .expect("Error during the websocket handshake occurred");
  println!("WebSocket connection established: {}", addr);

  // let subscription_id = Uuid::new_v4();

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
    let peers = peer_map.lock().unwrap();

    let mut mutable_events = events.lock().unwrap();
    let mut mutable_filters = filters.lock().unwrap();

    // Matches the type of structure that it's coming from the client.
    match serde_json::from_str::<Filter>(msg.to_string().as_str()) {
      Ok(filter) => {
        println!("{:?}", filter);
        mutable_filters.push(msg.to_string());
      }
      Err(e) => {
        println!("It's not a Filter")
      }
    }

    match serde_json::from_str::<Event>(msg.to_string().as_str()) {
      Ok(event) => {
        println!("{:?}", event);
        mutable_events.push(msg.to_string());
      }
      Err(e) => {
        println!("It's not an Event")
      }
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

#[tokio::main]
pub async fn initiate_relay() -> Result<(), IoError> {
  // thread-safe and lockable
  let events = Arc::new(Mutex::new(Vec::<String>::new()));
  let filters = Arc::new(Mutex::new(Vec::<String>::new()));

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

  // events.lock().unwrap().push(event_test);

  let addr = env::args()
    .nth(1)
    .unwrap_or_else(|| "127.0.0.1:8080".to_string());

  let state = PeerMap::new(Mutex::new(HashMap::new()));

  // Create the event loop and TCP listener we'll accept connections on.
  let try_socket = TcpListener::bind(&addr).await;
  let listener = try_socket.expect("Failed to bind");
  println!("Listening on: {}", addr);

  // Let's spawn the handling of each connection in a separate task.
  while let Ok((stream, addr)) = listener.accept().await {
    println!("New connection attempt!!!\n\n\n");
    tokio::spawn(handle_connection(
      state.clone(),
      stream,
      addr,
      events.clone(),
      filters.clone(),
    ));
  }

  Ok(())
}
