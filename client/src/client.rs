use bitcoin_hashes::hex::ToHex;
use std::{
  sync::Arc,
  time::{SystemTime, UNIX_EPOCH},
};
use tokio::{net::TcpStream, sync::Mutex};

use futures_channel::mpsc::UnboundedSender;
use futures_util::{
  stream::{FuturesUnordered, SplitStream},
  StreamExt,
};
use serde::{Deserialize, Serialize};
use tokio_tungstenite::{
  connect_async, tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream,
};

use log::{debug, error, info};
use uuid::Uuid;

use nostr_sdk::filter::Filter;
use nostr_sdk::{
  client_to_relay_communication::{
    event::ClientToRelayCommEvent, request::ClientToRelayCommRequest,
  },
  event::{kind::EventKind, Event},
};

use crate::db::{get_client_keys, Keys};

#[derive(Debug, Default, Serialize, Deserialize)]
struct Metadata {
  name: String,
  about: String,
  picture: String,
}

impl Metadata {
  pub fn as_str(&self) -> String {
    serde_json::to_string(self).unwrap()
  }
}

type WsRx = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;
#[derive(Debug, Default, Clone)]
struct RelayData {
  url: String,
  tx: Option<UnboundedSender<Message>>,
  rx: Option<Arc<Mutex<WsRx>>>,
}

#[derive(Debug, Default)]
pub struct Client {
  relays: Arc<Mutex<Vec<RelayData>>>,
  subscriptions_ids: Arc<Mutex<Vec<String>>>,
  keys: Keys,
  metadata: Metadata,
}

impl Client {
  pub fn new() -> Self {
    let keys = get_client_keys().unwrap();
    let default_relays: Vec<RelayData> = vec![
      RelayData {
        url: "ws://127.0.0.1:8080/".to_string(),
        tx: None,
        rx: None,
      },
      RelayData {
        url: "ws://127.0.0.1:8081/".to_string(),
        tx: None,
        rx: None,
      },
    ];

    Self {
      relays: Arc::new(Mutex::new(default_relays)),
      keys,
      subscriptions_ids: Arc::new(Mutex::new(Vec::<String>::new())),
      ..Default::default()
    }
  }

  pub fn name(&mut self, name: &str) -> &mut Self {
    self.metadata.name = name.to_string();
    self
  }

  pub fn about(&mut self, about: &str) -> &mut Self {
    self.metadata.about = about.to_string();
    self
  }

  pub fn picture(&mut self, picture: &str) -> &mut Self {
    self.metadata.picture = picture.to_string();
    self
  }

  pub async fn add_relay(&mut self, relay: String) {
    let mut relays = self.relays.lock().await;
    relays.push(RelayData {
      url: relay,
      tx: None,
      rx: None,
    });
  }

  pub async fn remove_relay(&mut self, relay: String) {
    let mut relays = self.relays.lock().await;
    relays.retain(|relay_data| *relay_data.url != relay);
  }

  fn get_timestamp_in_seconds(&self) -> u64 {
    let start = SystemTime::now();
    let since_the_epoch = start
      .duration_since(UNIX_EPOCH)
      .expect("Time went backwards");
    since_the_epoch.as_secs()
  }

  fn create_event(&self, kind: EventKind, content: String) -> Event {
    let pubkey = &self.keys.public_key.to_hex()[2..];
    let created_at = self.get_timestamp_in_seconds();
    let tags = vec![];

    let mut event =
      Event::new_without_signature(pubkey.to_string(), created_at, kind, tags, content);
    event.sign_event(self.keys.private_key.clone());
    event
  }

  pub fn publish_text_note(
    &self,
    note: String,
    tx: futures_channel::mpsc::UnboundedSender<Message>,
  ) {
    let to_publish = ClientToRelayCommEvent {
      event: self.create_event(EventKind::Text, note),
      ..Default::default()
    }
    .as_json();

    tx.unbounded_send(Message::binary(to_publish.as_bytes()))
      .unwrap();
  }

  pub fn get_event_metadata(&self) -> ClientToRelayCommEvent {
    ClientToRelayCommEvent {
      event: self.create_event(EventKind::Metadata, self.metadata.as_str()),
      ..Default::default()
    }
  }

  pub fn publish_metadata(&self, tx: futures_channel::mpsc::UnboundedSender<Message>) {
    let to_publish = self.get_event_metadata().as_json();
    publish_metadata(tx, to_publish);
  }

  pub async fn subscribe(
    &self,
    filters: Vec<Filter>,
    tx: futures_channel::mpsc::UnboundedSender<Message>,
  ) {
    let subscription_id = Uuid::new_v4().to_string();

    let filter_subscription = ClientToRelayCommRequest {
      filters,
      subscription_id: subscription_id.clone(),
      ..Default::default()
    }
    .as_json();

    // send via tx
    tx.unbounded_send(Message::binary(filter_subscription.as_bytes()))
      .unwrap();

    self.subscriptions_ids.lock().await.push(subscription_id);
  }

  pub fn close_connection(&self) {}

  #[tokio::main]
  pub async fn connect(self: Arc<Self>) {
    let relays_length = self.relays.lock().await.len();
    let mut threads: Vec<_> = vec![];
    for index in 0..relays_length {
      let current_relay = self.relays.lock().await;
      let current_relay = &current_relay[index];
      debug!("Connecting to relay {}", current_relay.url);
      let this = self.clone();
      let metadata = self.get_event_metadata();
      threads.push(tokio::spawn(handle_connection(
        this.relays.clone(),
        index,
        metadata,
      )));
    }

    let futures: FuturesUnordered<_> = threads.into_iter().collect();
    let _: Vec<_> = futures.collect().await;
  }

  #[tokio::main]
  pub async fn notifications(&self) {
    let relays = self.relays.lock().await;
    let mut notifications = Vec::new();

    for relay in relays.iter() {
      if let Some(ws_rx) = &relay.rx {
        let relay_url = relay.url.clone();
        let rx = Arc::clone(ws_rx);
        let notification = tokio::spawn(async move {
          let mut rx = rx.lock().await;
          while let Some(message) = rx.next().await {
            match message {
              Ok(msg) => {
                debug!(
                  "Received message from relay {}: {}",
                  relay_url,
                  msg.to_text().unwrap()
                );
              }
              Err(err) => {
                error!("[ws_to_stdout] {}", err);
              }
            }
          }
        });

        notifications.push(notification);
      }
    }

    drop(relays);

    let futures: FuturesUnordered<_> = notifications.into_iter().collect();
    let _: Vec<_> = futures.collect().await;
  }
}

async fn handle_connection(
  relays: Arc<Mutex<Vec<RelayData>>>,
  index: usize,
  metadata: ClientToRelayCommEvent,
) {
  let mut relay = relays.lock().await;
  let mut relay = &mut relay[index];
  let url = url::Url::parse(&relay.url).unwrap();
  let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
  info!(
    "WebSocket handshake to {} has been successfully completed",
    relay.url
  );

  let (ws_write, ws_read) = ws_stream.split();
  relay.rx = Some(Arc::new(Mutex::new(ws_read)));

  // mpsc stands for "multiple producers, single consumer"
  // unbounded is when the channel has infinity capacity
  // this unbounded channel is created to have different parts of the code
  // that can send messages to the sink (write) part of the websocket
  let (tx, rx) = futures_channel::mpsc::unbounded();
  relay.tx = Some(tx.clone());

  // send initial message
  publish_metadata(tx, metadata.as_json());

  // `tx` and `rx` are connected (two sides of the same channel)
  // when we receive something on the `rx` (in other words, we sent something using `tx`)
  // it will forward it to the write part of our websocket (`ws_write`) and this is where
  // the message will be forwarded to the server.
  let _ = rx.map(Ok).forward(ws_write).await;
}

fn publish_metadata(tx: futures_channel::mpsc::UnboundedSender<Message>, to_publish: String) {
  tx.unbounded_send(Message::binary(to_publish.as_bytes()))
    .unwrap();
}
