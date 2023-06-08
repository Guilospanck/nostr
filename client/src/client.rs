use bitcoin_hashes::hex::ToHex;
use std::{
  sync::Arc,
  time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::Mutex;

use serde::{Deserialize, Serialize};
use tokio_tungstenite::tungstenite::protocol::Message;

use uuid::Uuid;

use nostr_sdk::filter::Filter;
use nostr_sdk::{
  client_to_relay_communication::{
    event::ClientToRelayCommEvent, request::ClientToRelayCommRequest,
  },
  event::{kind::EventKind, Event},
};

use crate::{
  db::{get_client_keys, Keys},
  pool::RelayPool,
};

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

#[derive(Debug)]
pub struct Client {
  keys: Keys,
  metadata: Metadata,
  subscriptions_ids: Arc<Mutex<Vec<String>>>,
  pool: RelayPool,
}

impl Default for Client {
  fn default() -> Self {
    Self::new()
  }
}

impl Client {
  pub fn new() -> Self {
    let keys = get_client_keys().unwrap();

    let pool = RelayPool::new();

    Self {
      keys,
      subscriptions_ids: Arc::new(Mutex::new(Vec::<String>::new())),
      metadata: Metadata::default(),
      pool,
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
    self.pool.add_relay(relay).await;
  }

  pub async fn remove_relay(&mut self, relay: String) {
    self.pool.remove_relay(relay).await;
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

  pub async fn publish_text_note(
    &self,
    note: String,
  ) {
    let to_publish = ClientToRelayCommEvent {
      event: self.create_event(EventKind::Text, note),
      ..Default::default()
    }
    .as_json();

    self
      .pool
      .broadcast_messages(Message::binary(to_publish.as_bytes()))
      .await;
  }

  pub fn get_event_metadata(&self) -> String {
    ClientToRelayCommEvent {
      event: self.create_event(EventKind::Metadata, self.metadata.as_str()),
      ..Default::default()
    }
    .as_json()
  }

  pub async fn subscribe(&self, filters: Vec<Filter>) {
    let subscription_id = Uuid::new_v4().to_string();

    let filter_subscription = ClientToRelayCommRequest {
      filters,
      subscription_id: subscription_id.clone(),
      ..Default::default()
    }
    .as_json();

    // Broadcast subscription to all relays in the pool
    self
      .pool
      .broadcast_messages(Message::binary(filter_subscription.as_bytes()))
      .await;

    self.subscriptions_ids.lock().await.push(subscription_id);
  }

  pub fn close_connection(&self) {}

  pub async fn connect(&self) {
    self
      .pool
      .connect(Message::from(self.get_event_metadata()))
      .await;
  }
}
