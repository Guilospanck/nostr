pub mod communication_with_relay;
pub mod database;

use bitcoin_hashes::hex::ToHex;
use log::debug;
use std::{
  collections::HashMap,
  sync::Arc,
  time::{Duration, SystemTime, UNIX_EPOCH},
  vec,
};
use tokio::sync::{Mutex, MutexGuard};

use serde::{Deserialize, Serialize};
use tokio_tungstenite::tungstenite::protocol::Message;

use uuid::Uuid;

use crate::{
  client::{
    communication_with_relay::{
      close::ClientToRelayCommClose, event::ClientToRelayCommEvent,
      request::ClientToRelayCommRequest,
    },
    database::{
      keys_table::{Keys, KeysTable},
      subscriptions_table::SubscriptionsTable,
    },
  },
  event::{
    id::EventId,
    kind::EventKind,
    marker::Marker,
    tag::{Tag, UncheckedRecommendRelayURL},
    Event,
  },
  filter::Filter,
  relay::pool::RelayPool,
};

#[cfg(not(test))]
fn get_time_now() -> SystemTime {
  SystemTime::now()
}

#[allow(dead_code)]
const SECONDS_AFTER_UNIX_EPOCH_FOR_TIME_NOW_CONFIG_TEST: u64 = 20u64;
#[cfg(test)]
fn get_time_now() -> SystemTime {
  UNIX_EPOCH + Duration::new(SECONDS_AFTER_UNIX_EPOCH_FOR_TIME_NOW_CONFIG_TEST, 0)
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Metadata {
  pub name: String,
  pub about: String,
  pub picture: String,
}

impl Metadata {
  pub fn as_str(&self) -> String {
    serde_json::to_string(self).unwrap()
  }
}

#[derive(Debug)]
pub struct Client {
  keys: Keys,
  pub metadata: Metadata,
  subscriptions: Arc<Mutex<HashMap<String, Vec<Filter>>>>,
  subscriptions_db: SubscriptionsTable,
  pool: RelayPool,
}

impl Default for Client {
  fn default() -> Self {
    Self::new(None, None)
  }
}

impl Client {
  pub fn new(keys_table_name: Option<String>, subscriptions_table_name: Option<String>) -> Self {
    let keys = KeysTable::new(keys_table_name).get_client_keys().unwrap();
    let subscriptions_db = SubscriptionsTable::new(subscriptions_table_name);
    let subscriptions = subscriptions_db.get_all_subscriptions().unwrap();

    let pool = RelayPool::new();

    Self {
      keys,
      subscriptions: Arc::new(Mutex::new(subscriptions)),
      subscriptions_db,
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

  /// Adds relay to the pool
  /// (and automatically connects to it and sends client metadata).
  pub async fn add_relay(&mut self, relay: String) {
    self
      .pool
      .add_relay(
        relay.clone(),
        Message::from(self.get_event_metadata().as_json()),
      )
      .await;
  }

  /// This function has the same semantics as `crate::relay::pool::RelayPool.remove_relay()`.
  pub async fn remove_relay(&mut self, relay: String) {
    self.pool.remove_relay(relay).await;
  }

  fn get_timestamp_in_seconds(&self) -> u64 {
    let start = get_time_now();
    let since_the_epoch: Duration = start
      .duration_since(UNIX_EPOCH)
      .expect("Time went backwards");
    since_the_epoch.as_secs()
  }

  pub fn get_hex_public_key(&self) -> String {
    self.keys.public_key.to_hex()
  }

  fn create_event(&self, kind: EventKind, content: String, tags: Option<Vec<Tag>>) -> Event {
    let pubkey = self.keys.public_key.to_hex();
    let created_at = self.get_timestamp_in_seconds();
    let tags = tags.unwrap_or(vec![]);

    let mut event = Event::new_without_signature(pubkey, created_at, kind, tags, content);
    event.sign_event(self.keys.private_key.clone());
    event
  }

  pub fn create_reply_to_event(
    &self,
    event_referenced: Event,
    recommended_relay_url: Option<UncheckedRecommendRelayURL>,
    marker: Marker,
    content: String,
  ) -> ClientToRelayCommEvent {
    let event_id_referenced = EventId(event_referenced.id);
    let recommended_relay = recommended_relay_url.unwrap_or(UncheckedRecommendRelayURL::default());

    // e tags
    let e_tag = Tag::Event(event_id_referenced, Some(recommended_relay), Some(marker));

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

    ClientToRelayCommEvent {
      event: self.create_event(EventKind::Text, content, Some(tags)),
      ..Default::default()
    }
  }

  pub fn create_text_note_event(&self, note: String) -> ClientToRelayCommEvent {
    ClientToRelayCommEvent {
      event: self.create_event(EventKind::Text, note, None),
      ..Default::default()
    }
  }

  pub fn get_event_metadata(&self) -> ClientToRelayCommEvent {
    ClientToRelayCommEvent {
      event: self.create_event(EventKind::Metadata, self.metadata.as_str(), None),
      ..Default::default()
    }
  }

  fn get_filter_subscription_request(&self, filters: Vec<Filter>) -> ClientToRelayCommRequest {
    let subscription_id = Uuid::new_v4().to_string();

    ClientToRelayCommRequest {
      filters,
      subscription_id,
      ..Default::default()
    }
  }

  pub async fn subscribe(&self, filters: Vec<Filter>) {
    let filter_subscription = self.get_filter_subscription_request(filters.clone());

    debug!("SUBSCRIBING to {:?}", filter_subscription);

    // Broadcast REQ subscription to all relays in the pool
    self
      .pool
      .broadcast_messages(Message::from(filter_subscription.as_json()))
      .await;

    // save to db
    let filters_string = serde_json::to_string(&filters).unwrap();
    self
      .subscriptions_db
      .add_new_subscription(&filter_subscription.subscription_id, &filters_string);

    // save to memory
    self
      .subscriptions_mut()
      .await
      .insert(filter_subscription.subscription_id, filters);
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
    self.subscriptions_db.remove_subscription(subscription_id);

    // remove from memory
    self.subscriptions_mut().await.remove(subscription_id);
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

  pub async fn subscriptions_mut(&self) -> MutexGuard<HashMap<String, Vec<Filter>>> {
    self.subscriptions.lock().await
  }

  pub async fn send_updated_metadata(&self) {
    self
      .pool
      .broadcast_messages(Message::from(self.get_event_metadata().as_json()))
      .await;
  }

  pub async fn publish(&self, to_publish: String) {
    self
      .pool
      .broadcast_messages(Message::from(to_publish))
      .await;
  }

  pub async fn close_connection(&self, relay_url: String) {
    self.pool.disconnect_relay(relay_url).await;
  }

  pub async fn connect(&self) {
    self
      .pool
      .connect(Message::from(self.get_event_metadata().as_json()))
      .await;
  }

  pub async fn get_notifications(&self) {
    self.pool.notifications().await;
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[cfg(test)]
  use pretty_assertions::assert_eq;
  use serde_json::json;
  use std::fs;

  fn remove_temp_db(table_name: &str) {
    fs::remove_file(format!("db/{table_name}.redb")).unwrap();
  }

  #[test]
  fn metadata() {
    // arrange
    let name = "Client test";
    let about = "Client about";
    let picture = "client.picture.com";

    let mut client = Client::new(Some("metadata".to_string()), Some("metadata".to_string()));

    // act
    client.name(name).about(about).picture(picture);

    // assert
    assert_eq!(client.metadata.name, name.to_string());
    assert_eq!(client.metadata.about, about.to_string());
    assert_eq!(client.metadata.picture, picture.to_string());

    client
      .name("potato")
      .about("anotherpotato")
      .picture("picturepotato");
    assert_eq!(client.metadata.name, "potato".to_string());
    assert_eq!(client.metadata.about, "anotherpotato".to_string());
    assert_eq!(client.metadata.picture, "picturepotato".to_string());

    remove_temp_db("metadata");
  }

  #[tokio::test]
  async fn add_and_remove_relay() {
    // arrange
    let relay = "relay1".to_string();
    let mut client = Client::new(
      Some("add_remove_relay".to_string()),
      Some("add_remove_relay".to_string()),
    );

    client.add_relay(relay.clone()).await;
    assert_eq!(client.pool.relays().await.len(), 1);

    client.remove_relay(relay).await;
    assert!(client.pool.relays().await.is_empty());

    remove_temp_db("add_remove_relay");
  }

  #[test]
  fn get_timestamp_in_seconds() {
    let client = Client::new(Some("timestamp".to_string()), Some("timestamp".to_string()));
    let timestamp = client.get_timestamp_in_seconds();
    assert_eq!(timestamp, SECONDS_AFTER_UNIX_EPOCH_FOR_TIME_NOW_CONFIG_TEST);

    remove_temp_db("timestamp");
  }

  #[test]
  fn create_event() {
    let client = Client::new(
      Some("create_event".to_string()),
      Some("create_event".to_string()),
    );
    let kind = EventKind::Text;
    let content = String::from("Content test");
    let tags = None;

    let event = client.create_event(kind, content.clone(), tags);

    assert_eq!(event.content, content);
    assert_eq!(event.kind, kind);
    assert_eq!(event.tags, []);
    assert_eq!(event.pubkey, client.get_hex_public_key());
    assert_eq!(
      event.created_at,
      SECONDS_AFTER_UNIX_EPOCH_FOR_TIME_NOW_CONFIG_TEST
    );

    remove_temp_db("create_event");
  }

  #[test]
  fn create_reply_to_event() {
    let client = Client::new(
      Some("create_reply_to_event".to_string()),
      Some("create_reply_to_event".to_string()),
    );
    let kind = EventKind::Text;
    let content = String::from("Content test");
    let tags = None;
    let event = client.create_event(kind, content, tags);

    let recommended_relay_url = None;
    let content_for_reply = String::from("Replying to event");
    let marker = Marker::Root;
    let replyed_event = client.create_reply_to_event(
      event.clone(),
      recommended_relay_url,
      marker.clone(),
      content_for_reply.clone(),
    );

    let tags_json_string = serde_json::to_string(&replyed_event.event.tags).unwrap();

    let expected_tags = json!([
      [
        "e".to_string(),
        event.id,
        "".to_string(),
        marker.to_string()
      ],
      ["p".to_string(), client.get_hex_public_key()]
    ])
    .to_string();

    assert_eq!(replyed_event.event.content, content_for_reply);
    assert_eq!(tags_json_string, expected_tags);
    assert_eq!(
      event.created_at,
      SECONDS_AFTER_UNIX_EPOCH_FOR_TIME_NOW_CONFIG_TEST
    );

    remove_temp_db("create_reply_to_event");
  }

  #[test]
  fn create_text_note_event() {
    let client = Client::new(
      Some("create_text_note_event".to_string()),
      Some("create_text_note_event".to_string()),
    );
    let note = String::from("Test Note");

    let text_note_event = client.create_text_note_event(note.clone());

    assert_eq!(text_note_event.event.content, note);
    assert_eq!(text_note_event.event.kind, EventKind::Text);
    assert_eq!(
      text_note_event.event.created_at,
      SECONDS_AFTER_UNIX_EPOCH_FOR_TIME_NOW_CONFIG_TEST
    );

    remove_temp_db("create_text_note_event");
  }

  #[test]
  fn get_event_metadata() {
    let client = Client::new(
      Some("get_event_metadata".to_string()),
      Some("get_event_metadata".to_string()),
    );

    let metadata_event = client.get_event_metadata();

    assert_eq!(metadata_event.event.content, client.metadata.as_str());
    assert_eq!(metadata_event.event.kind, EventKind::Metadata);
    assert_eq!(
      metadata_event.event.created_at,
      SECONDS_AFTER_UNIX_EPOCH_FOR_TIME_NOW_CONFIG_TEST
    );

    remove_temp_db("get_event_metadata");
  }

  #[test]
  fn get_filter_subscription_request() {
    let client = Client::new(
      Some("get_filter_subscription_request".to_string()),
      Some("get_filter_subscription_request".to_string()),
    );
    let filter = Filter::default();
    let metadata_event = client.get_filter_subscription_request(vec![filter.clone()]);

    assert_eq!(metadata_event.filters, vec![filter]);
    assert_eq!(metadata_event.code, String::from("REQ"));

    remove_temp_db("get_filter_subscription_request");
  }

  #[tokio::test]
  async fn subscribe_and_unsubcribe() {
    let client = Client::new(
      Some("subscribe_and_unsubcribe".to_string()),
      Some("subscribe_and_unsubcribe".to_string()),
    );
    // Initial
    let subscriptions = client.subscriptions().await;
    let subscriptions_from_db = client.subscriptions_db.get_all_subscriptions().unwrap();
    assert_eq!(subscriptions.len(), 0);
    assert_eq!(subscriptions_from_db.len(), 0);

    // subscribe
    let filter = Filter::default();
    client.subscribe(vec![filter]).await;

    // after subscription
    let subscriptions = client.subscriptions().await;
    let subscriptions_from_db = client.subscriptions_db.get_all_subscriptions().unwrap();
    assert_eq!(subscriptions.len(), 1);
    assert_eq!(subscriptions_from_db.len(), 1);

    // unsubscribe
    let subscription_id = subscriptions.keys().next().unwrap();
    client.unsubscribe(subscription_id).await;

    // after unsubscribtion
    let subscriptions = client.subscriptions().await;
    let subscriptions_from_db = client.subscriptions_db.get_all_subscriptions().unwrap();
    assert_eq!(subscriptions.len(), 0);
    assert_eq!(subscriptions_from_db.len(), 0);

    remove_temp_db("subscribe_and_unsubcribe");
  }
}
