use std::sync::{Arc, Mutex};

use futures_util::{future, pin_mut, stream::FuturesUnordered, StreamExt};
use tokio::io::AsyncReadExt;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

use uuid::Uuid;
use log::{debug, info, error};

use nostr_sdk::client_to_relay_communication::request::ClientToRelayCommRequest;
use nostr_sdk::event::id::EventId;
use nostr_sdk::filter::Filter;

use crate::db::{get_client_keys, Keys};

const LIST_OF_RELAYS: [&str; 2] = [
  // "wss://nostr-relay.guilospanck.com",
  "ws://127.0.0.1:8080/",
  "ws://127.0.0.1:8081/",
];

/// Our helper method which will read data from stdin and send it along the
/// sender provided.
/// Send STDIN to WS
/// 
async fn read_stdin(tx: futures_channel::mpsc::UnboundedSender<Message>) {
  let mut stdin = tokio::io::stdin();
  loop {
    let mut buf = vec![0; 1024];
    let n = match stdin.read(&mut buf).await {
      Err(_) | Ok(0) => break,
      Ok(n) => n,
    };
    buf.truncate(n);
    tx.unbounded_send(Message::binary(buf)).unwrap();
  }
}

/// Our helper method which will send initial data upon connection.
/// It will require some data from the relay using a filter subscription.
/// 
async fn send_initial_message(
  tx: futures_channel::mpsc::UnboundedSender<Message>,
  subscriptions_ids: Arc<Mutex<Vec<String>>>,
) {
  let filters = vec![Filter {
    ids: Some([EventId("05b25af3-4250-4fbf-8ef5-97220858f9ab".to_owned())].to_vec()),
    authors: None,
    kinds: None,
    e: None,
    p: None,
    since: None,
    until: None,
    limit: None,
  }];

  let subscription_id = Uuid::new_v4().to_string();

  let mut subs_id = subscriptions_ids.lock().unwrap();
  subs_id.push(subscription_id.clone());

  // ["REQ","some-random-subs-id",{"ids":["ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb"]},{"authors":["5081ce98f7da142513444079a55e2d1676559a908d4f694d299057f8abddf835"]}]
  // ["EVENT",{"id":"ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb","pubkey":"02c7e1b1e9c175ab2d100baf1d5a66e73ecc044e9f8093d0c965741f26aa3abf76","created_at":1673002822,"kind":1,"tags":[["e","688787d8ff144c502c7f5cffaafe2cc588d86079f9de88304c26b0cb99ce91c6","wss://relay.damus.io"],["p","02c7e1b1e9c175ab2d100baf1d5a66e73ecc044e9f8093d0c965741f26aa3abf76",""]],"content":"Lorem ipsum dolor sit amet","sig":"e8551d85f530113366e8da481354c2756605e3f58149cedc1fb9385d35251712b954af8ef891cb0467d50ddc6685063d4190c97e9e131f903e6e4176dc13ce7c"}]
  // ["EVENT",{"id":"05b25af3-4250-4fbf-8ef5-97220858f9ab","pubkey":"02c7e1b1e9c175ab2d100baf1d5a66e73ecc044e9f8093d0c965741f26aa3abf76","created_at":1673002822,"kind":1,"tags":[["e","688787d8ff144c502c7f5cffaafe2cc588d86079f9de88304c26b0cb99ce91c6","wss://relay.damus.io"],["p","02c7e1b1e9c175ab2d100baf1d5a66e73ecc044e9f8093d0c965741f26aa3abf76",""]],"content":"Lorem ipsum dolor sit amet","sig":"e8551d85f530113366e8da481354c2756605e3f58149cedc1fb9385d35251712b954af8ef891cb0467d50ddc6685063d4190c97e9e131f903e6e4176dc13ce7c"}]
  // ["CLOSE","95e1c438-133d-428d-a849-a307c2e1a005"]
  let filter_subscription = ClientToRelayCommRequest {
    filters,
    subscription_id,
    ..Default::default()
  }.as_json();

  tx.unbounded_send(Message::binary(filter_subscription.as_bytes()))
    .unwrap();
}

async fn handle_connection(
  connect_addr: String,
  subscriptions_ids: Arc<Mutex<Vec<String>>>,
  _keys: Keys,
) {
  let url = url::Url::parse(&connect_addr).unwrap();

  let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
  info!("WebSocket handshake has been successfully completed");

  let (tx, rx) = futures_channel::mpsc::unbounded();

  let (outgoing, incoming) = ws_stream.split();

  // Spawn new thread to read from stdin and send to relay.
  tokio::spawn(read_stdin(tx.clone()));

  // send initial message
  send_initial_message(tx, subscriptions_ids).await;

  let stdin_to_ws = rx.map(Ok).forward(outgoing);

  // This will print to stdout whatever the WS sends
  // (The WS is forwarding messages from other clients)
  let ws_to_stdout = {
    incoming.for_each(|message| async {
      match message {
        Ok(msg) => {
          debug!("Received message from relay: {}", msg.to_text().unwrap());
        }
        Err(err) => {
          error!("[ws_to_stdout] {err}");
        }
      }
    })
  };

  pin_mut!(stdin_to_ws, ws_to_stdout);
  future::select(stdin_to_ws, ws_to_stdout).await;
}

#[tokio::main]
pub async fn initiate_client() -> Result<(), redb::Error> {
  let keys = get_client_keys()?;

  let subscriptions_ids = Arc::new(Mutex::new(Vec::<String>::new()));

  let connections: Vec<_> = LIST_OF_RELAYS
    .into_iter()
    .map(|addr| {
      debug!("Connecting to relay {addr}");
      tokio::spawn(handle_connection(
        addr.to_string(),
        subscriptions_ids.clone(),
        keys.clone(),
      ))
    })
    .collect();

  let futures: FuturesUnordered<_> = connections.into_iter().collect();

  let _: Vec<_> = futures.collect().await;

  Ok(())
}
