use std::{
  sync::{Arc, Mutex},
  time::{SystemTime, UNIX_EPOCH},
};

use futures_util::{future, pin_mut, StreamExt};
use serde::{Deserialize, Serialize};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

use log::{debug, error, info};
use uuid::Uuid;

use nostr_sdk::event::id::EventId;
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

#[derive(Debug, Default)]
pub struct Client {
  relays: Arc<Mutex<Vec<String>>>,
  subscriptions_ids: Arc<Mutex<Vec<String>>>,
  keys: Keys,
  metadata: Metadata,
}

impl Client {
  pub fn new() -> Self {
    let keys = get_client_keys().unwrap();
    let default_relays: Vec<String> = vec![
      "ws://127.0.0.1:8080/".to_string(),
      "ws://127.0.0.1:8081/".to_string(),
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

  pub fn add_relay(&mut self, relay: String) {
    let mut relays = self.relays.lock().unwrap();
    relays.push(relay);
  }

  pub fn remove_relay(&mut self, relay: String) {
    let mut relays = self.relays.lock().unwrap();
    relays.retain(|addr| *addr != relay);
  }

  fn get_timestamp_in_seconds(&self) -> u64 {
    let start = SystemTime::now();
    let since_the_epoch = start
      .duration_since(UNIX_EPOCH)
      .expect("Time went backwards");
    since_the_epoch.as_secs()
  }

  fn create_event(&self, kind: EventKind, content: String) -> Event {
    let pubkey = &self.keys.public_key.to_string()[2..];
    let created_at = self.get_timestamp_in_seconds();
    let tags = vec![];

    let mut event =
      Event::new_without_signature(pubkey.to_string(), created_at, kind, tags, content);
    event.sign_event(self.keys.private_key.as_bytes().to_vec());
    event
  }

  pub fn publish_text_note(&self, note: String) {
    let _to_publish = ClientToRelayCommEvent {
      event: self.create_event(EventKind::Text, note),
      ..Default::default()
    };
  }

  pub fn publish_metadata(&self) {
    let _to_publish = ClientToRelayCommEvent {
      event: self.create_event(EventKind::Metadata, self.metadata.as_str()),
      ..Default::default()
    };
  }

  pub fn subscribe(&self, filters: Vec<Filter>) {
    let subscription_id = Uuid::new_v4().to_string();

    let _filter_subscription = ClientToRelayCommRequest {
      filters,
      subscription_id,
      ..Default::default()
    }
    .as_json();
  }

  pub fn close_connection(&self) {}

  async fn _handle_connection(&self, relay: String) {
    let url = url::Url::parse(&relay).unwrap();

    let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
    info!("WebSocket handshake to {relay} has been successfully completed");

    let (tx, rx) = futures_channel::mpsc::unbounded();

    let (outgoing, incoming) = ws_stream.split();

    // send initial message
    send_initial_message(tx, self.subscriptions_ids.clone()).await;

    let stdin_to_ws = rx.map(Ok).forward(outgoing);

    // This will print to stdout whatever the WS sends
    // (The WS is forwarding messages from other clients)
    let ws_to_stdout = {
      incoming.for_each(|message| async {
        match message {
          Ok(msg) => {
            debug!(
              "Received message from relay {relay}: {}",
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
  pub async fn connect(self) {
    for relay in self.relays.lock().unwrap().iter() {
      debug!("Connecting to relay {relay}");
      // tokio::spawn(self.handle_connection(relay.to_string()));
    }
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
