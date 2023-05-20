use std::{
  env,
  sync::{Arc, Mutex},
};

use futures_util::{future, pin_mut, stream::FuturesUnordered, StreamExt};
use tokio::io::AsyncReadExt;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

use log::{debug, error, info};
use uuid::Uuid;

use nostr_sdk::client_to_relay_communication::request::ClientToRelayCommRequest;
use nostr_sdk::event::id::EventId;
use nostr_sdk::filter::Filter;

use crate::db::{get_client_keys, Keys};

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

  let filter_subscription = ClientToRelayCommRequest {
    filters,
    subscription_id,
    ..Default::default()
  }
  .as_json();

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
  info!("WebSocket handshake to {connect_addr} has been successfully completed");

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
          debug!(
            "Received message from relay {connect_addr}: {}",
            msg.to_text().unwrap()
          );
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

  let relays_list = env::var("RELAY_LIST")
    .as_ref()
    .map(|list| {
      let splitted: Vec<String> = list
        .split(',')
        .map(|ws_relay| ws_relay.to_string())
        .collect();
      splitted
    })
    .unwrap_or_else(|_| vec!["ws://127.0.0.1:8080/".to_string()]);

  let connections: Vec<_> = relays_list
    .iter()
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
