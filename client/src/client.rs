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

use nostr_sdk::{
  client_to_relay_communication::close::ClientToRelayCommClose,
  event::{
    id::EventId,
    marker::Marker,
    tag::{Tag, UncheckedRecommendRelayURL},
  },
  filter::Filter,
};
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

  pub fn get_hex_public_key(&self) -> String {
    self.keys.public_key.to_hex()
  }

  // TODO: put this method back to private
  pub fn create_event(&self, kind: EventKind, content: String, tags: Option<Vec<Tag>>) -> Event {
    let pubkey = self.keys.public_key.to_hex();
    let created_at = self.get_timestamp_in_seconds();
    let tags = tags.unwrap_or(vec![]);

    let mut event = Event::new_without_signature(pubkey, created_at, kind, tags, content);
    event.sign_event(self.keys.private_key.clone());
    event
  }

  pub async fn reply_to_event(
    &self,
    event_referenced: Event,
    recommended_relay_url: Option<UncheckedRecommendRelayURL>,
    marker: Marker,
    content: String,
  ) {
    let event_id_referenced = EventId(event_referenced.id);
    let recommended_relay = recommended_relay_url.unwrap_or(UncheckedRecommendRelayURL::default());

    // e tags
    let e_tag = Tag::Event(
      event_id_referenced,
      Some(recommended_relay.clone()),
      Some(marker),
    );

    // whenever replying to an event, the p tag should have at least the pubkey of the creator of the event
    let mut pubkeys_from_event_referenced: Vec<String> = vec![event_referenced.pubkey];
    for tag in event_referenced.tags {
      if let Tag::PubKey(event_pubkey_tag_pubkey, _) = tag {
        if !event_pubkey_tag_pubkey.is_empty() {
          pubkeys_from_event_referenced.extend_from_slice(&event_pubkey_tag_pubkey);
        }
      }
    }

    let p_tag = Tag::PubKey(pubkeys_from_event_referenced, None);

    let tags = vec![e_tag, p_tag];

    let to_publish = ClientToRelayCommEvent {
      event: self.create_event(EventKind::Text, content, Some(tags)),
      ..Default::default()
    }
    .as_json();

    self.publish(to_publish).await
  }

  pub async fn publish_text_note(&self, note: String) {
    let to_publish = ClientToRelayCommEvent {
      event: self.create_event(EventKind::Text, note, None),
      ..Default::default()
    }
    .as_json();

    self.publish(to_publish).await;
  }

  pub async fn publish(&self, to_publish: String) {
    self
      .pool
      .broadcast_messages(Message::from(to_publish))
      .await;
  }

  pub fn get_event_metadata(&self) -> String {
    ClientToRelayCommEvent {
      event: self.create_event(EventKind::Metadata, self.metadata.as_str(), None),
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

    debug!("SUBSCRIBING to {:?}", filter_subscription);

    // Broadcast REQ subscription to all relays in the pool
    self
      .pool
      .broadcast_messages(Message::from(filter_subscription))
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

  pub async fn unsubscribe(&self, subscription_id: &str) {
    let close_subscription = ClientToRelayCommClose {
      subscription_id: subscription_id.to_string(),
      ..Default::default()
    }
    .as_json();

    // Broadcast CLOSE subscription to all relays in the pool
    self
      .pool
      .broadcast_messages(Message::from(close_subscription))
      .await;

    // remove from db
    SubscriptionsTable::new().remove_subscription(subscription_id);

    // remove from memory
    let mut subscriptions = self.subscriptions().await;
    subscriptions.remove(subscription_id);
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
        .broadcast_messages(Message::from(filter_subscription))
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

  pub async fn follow_myself(&self) {
    let pubkey = self.keys.public_key.to_hex();
    let filter = Filter {
      authors: Some(vec![pubkey]),
      ..Default::default()
    };

    self.subscribe(vec![filter]).await;
  }

  pub async fn subscriptions(&self) -> HashMap<String, Vec<Filter>> {
    let subscriptions = self.subscriptions.lock().await;
    subscriptions.clone()
  }

  pub async fn close_connection(&self, relay_url: String) {
    self.pool.disconnect_relay(relay_url).await;
  }

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
