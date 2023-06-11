use bitcoin_hashes::hex::ToHex;
use log::debug;
use std::{
  collections::HashMap,
  sync::Arc,
  time::{SystemTime, UNIX_EPOCH},
  vec,
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
  database::{
    keys_table::{Keys, KeysTable},
    subscriptions_table::SubscriptionsTable,
  },
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
  subscriptions: Arc<Mutex<HashMap<String, Vec<Filter>>>>,
  pool: RelayPool,
}

impl Default for Client {
  fn default() -> Self {
    Self::new()
  }
}

impl Client {
  pub fn new() -> Self {
    let keys = KeysTable::new().get_client_keys().unwrap();
    let subscriptions = SubscriptionsTable::new().get_all_subscriptions().unwrap();
    debug!("{:?}", subscriptions);

    let pool = RelayPool::new();

    Self {
      keys,
      subscriptions: Arc::new(Mutex::new(subscriptions)),
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
    self
      .pool
      .add_relay(relay.clone(), Message::from(self.get_event_metadata()))
      .await;
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
    let pubkey = &self.keys.public_key.to_hex();
    let created_at = self.get_timestamp_in_seconds();
    let tags = vec![];

    let mut event =
      Event::new_without_signature(pubkey.to_string(), created_at, kind, tags, content);
    event.sign_event(self.keys.private_key.clone());
    event
  }

  pub async fn publish_text_note(&self, note: String) {
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

  pub async fn send_updated_metadata(&self) {
    self
      .pool
      .broadcast_messages(Message::from(self.get_event_metadata()))
      .await;
  }

  pub async fn subscribe(&self, filters: Vec<Filter>) {
    let subscription_id = Uuid::new_v4().to_string();

    let filter_subscription = ClientToRelayCommRequest {
      filters: filters.clone(),
      subscription_id: subscription_id.clone(),
      ..Default::default()
    }
    .as_json();

    // Broadcast subscription to all relays in the pool
    self
      .pool
      .broadcast_messages(Message::binary(filter_subscription.as_bytes()))
      .await;

    // save to db
    let filters_string = serde_json::to_string(&filters).unwrap();
    SubscriptionsTable::new().add_new_subscription(&subscription_id, &filters_string);

    // save to memory
    self
      .subscriptions
      .lock()
      .await
      .insert(subscription_id, filters);
  }

  pub async fn subscribe_to_all_stored_requests(&self) {
    let subscriptions = self.subscriptions().await;

    for (subs_id, filters) in subscriptions.iter() {
      let filter_subscription = ClientToRelayCommRequest {
        filters: filters.clone(),
        subscription_id: subs_id.clone(),
        ..Default::default()
      }
      .as_json();

      // Broadcast subscription to all relays in the pool
      self
        .pool
        .broadcast_messages(Message::binary(filter_subscription.as_bytes()))
        .await;
    }
  }

  pub async fn follow_author(&self, author_pubkey: String) {
    let filter = Filter {
      authors: Some(vec![author_pubkey]),
      ..Default::default()
    };

    self.subscribe(vec![filter]).await;
  }

  pub async fn subscriptions(&self) -> HashMap<String, Vec<Filter>> {
    let subscriptions = self.subscriptions.lock().await;
    subscriptions.clone()
  }

  pub fn close_connection(&self) {}

  pub async fn connect(&self) {
    self
      .pool
      .connect(Message::from(self.get_event_metadata()))
      .await;
  }

  pub async fn get_notifications(&self) {
    self.pool.notifications().await;
  }
}
