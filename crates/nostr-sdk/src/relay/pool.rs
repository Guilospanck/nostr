use std::sync::atomic::{AtomicBool, Ordering};
use std::{collections::HashMap, sync::Arc};

use crate::relay::communication_with_client::{
  eose::RelayToClientCommEose, event::RelayToClientCommEvent, notice::RelayToClientCommNotice,
};
use futures_util::SinkExt;
use futures_util::StreamExt;
use log::debug;
use log::error;
use log::info;
use tokio::sync::MutexGuard;
use tokio::sync::{
  mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
  Mutex,
};
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[derive(Debug)]
pub enum RelayPoolMessage {
  /// Relay received some that was forwarded from another client
  ReceivedMsg { relay_url: String, msg: Message },
}

type PoolTaskSender = tokio::sync::mpsc::UnboundedSender<RelayPoolMessage>;

#[derive(Debug, Clone)]
pub struct RelayData {
  /// Url to connect to this relay.
  url: String,
  /// Tx used to send all messages received of this relay (from another client) to the pool.
  pool_task_sender: PoolTaskSender,
  /// Tx part of the channel to send messages (by this client) to this relay.
  relay_tx: UnboundedSender<Message>,
  /// Rx part of the channel to receive messages (by this client) from this relay.
  relay_rx: Arc<Mutex<UnboundedReceiver<Message>>>,
  /// Flag to signal if the connection must be closed
  close_communication: Arc<AtomicBool>,
  /// Flag to signal if the relay is already connected
  is_connected: Arc<AtomicBool>,
}

impl RelayData {
  fn new(url: String, pool_task_sender: PoolTaskSender) -> Self {
    let (relay_tx, relay_rx) = unbounded_channel();
    let close_communication = Arc::new(AtomicBool::new(false));
    let is_connected = Arc::new(AtomicBool::new(false));

    Self {
      url,
      pool_task_sender,
      relay_tx,
      relay_rx: Arc::new(Mutex::new(relay_rx)),
      close_communication,
      is_connected,
    }
  }

  async fn connect(&self, metadata: Message) {
    debug!("❯ Connecting to {}", self.url.clone());

    let connection = connect_async(self.url.clone()).await;

    // Connect
    match connection {
      Ok((ws_stream, _)) => {
        info!("❯ Connected to {}", self.url.clone());
        self.is_connected.store(true, Ordering::Relaxed);
        let (mut ws_tx, mut ws_rx) = ws_stream.split();

        // Send metadata on connection
        ws_tx.send(metadata).await.unwrap();
        debug!("Metadata sent to relay");

        // Whatever we receive from the relay (that was sent by other clients),
        // we'll send to the pool.
        // Check `RelayPoolTask.run` method to see where all messages
        // forwarded to the pool end up.
        let relay = self.clone();
        tokio::spawn(async move {
          debug!("❯ Relay Message Thread Started");

          while let Some(msg_res) = ws_rx.next().await {
            if let Ok(msg) = msg_res {
              relay
                .pool_task_sender
                .send(RelayPoolMessage::ReceivedMsg {
                  relay_url: relay.url.clone(),
                  msg,
                })
                .unwrap();
            }
          }

          debug!("❯ Exited from Message Thread of {}", relay.url);
        });

        // Send messages sent to this relay, which were sent by our client.
        let relay = self.clone();
        tokio::spawn(async move {
          let mut rx = relay.relay_rx.lock().await;
          while let Some(msg) = rx.recv().await {
            if relay.close_communication.load(Ordering::Relaxed) {
              break;
            }
            let _ = ws_tx.send(msg).await;
          }
          // Closes WS connection when `relay.close_communication` is true
          let _ = ws_tx.close().await;
        });
      }
      Err(err) => {
        error!("Impossible to connect to {}: {}", self.url, err);
      }
    };
  }

  fn disconnect(&self) {
    debug!("❯ Disconnecting from {}", self.url);
    self.close_communication.store(true, Ordering::Relaxed);
    self.is_connected.store(false, Ordering::Relaxed);
  }

  fn send_message(&self, message: Message) {
    self.relay_tx.send(message).unwrap()
  }
}

#[derive(Debug)]
pub struct RelayPool {
  relays: Arc<Mutex<HashMap<String, RelayData>>>,
  pool_task_sender: PoolTaskSender,
  relay_pool_task: RelayPoolTask,
}

impl Default for RelayPool {
  fn default() -> Self {
    Self::new()
  }
}

impl RelayPool {
  pub fn new() -> Self {
    // create channel to allow relays to communicate with the pool
    let (pool_task_sender, pool_task_receiver) = tokio::sync::mpsc::unbounded_channel();

    // creates the pool task in order to handle messages sent to it
    let relay_pool_task = RelayPoolTask::new(pool_task_receiver);

    // creates arc mutex of hashmap of relays
    let relays = Arc::new(Mutex::new(HashMap::new()));

    Self {
      relays,
      pool_task_sender,
      relay_pool_task,
    }
  }

  /// Gets a `read` version of the HashMap of relays.
  ///
  /// This is fine if you want to just read the contents of the HashMap of relays.
  /// But not if you want to mutate it.
  ///
  pub async fn relays(&self) -> HashMap<String, RelayData> {
    let relays = self.relays.lock().await;
    relays.clone()
  }

  /// Gets a `mutable` version of the HashMap of relays.
  ///
  /// This is ideal if you want to change the contents of the HashMap of relays.
  ///
  pub async fn relays_mut(&self) -> MutexGuard<HashMap<String, RelayData>> {
    self.relays.lock().await
  }

  /// Add relay to the pool hashmap and tries to connect to it
  /// if it does not already exist.
  ///
  pub async fn add_relay(&self, url: String, metadata: Message) {
    let mut relays = self.relays_mut().await;

    if relays.get(&url).is_none() {
      let relay = RelayData::new(url.clone(), self.pool_task_sender.clone());
      relays.insert(url, relay.clone());
      relay.connect(metadata).await;
    }
  }

  /// Removes from the pool and disconnects from the relay.
  ///
  pub async fn remove_relay(&self, url: String) {
    let mut relays = self.relays_mut().await;
    if relays.contains_key(&url) {
      relays[&url].disconnect();
      relays.remove(&url);
    }
  }

  /// Connects to all relays in the pool that are not yet connected.
  ///
  pub async fn connect(&self, metadata: Message) {
    let relays = self.relays().await;
    for relay in relays.values() {
      if !relay.is_connected.load(Ordering::Relaxed) {
        relay.connect(metadata.clone()).await;
      }
    }
  }

  /// Disconnects from a relay (does not remove it from the pool).
  ///
  pub async fn disconnect_relay(&self, relay_url: String) {
    let relays = self.relays().await;
    if let Some(relay) = relays.get(&relay_url) {
      relay.disconnect();
    };
  }

  pub async fn notifications(&self) {
    let mut relay_pool_task = self.relay_pool_task.clone();
    tokio::spawn(async move { relay_pool_task.run().await });
  }

  pub async fn broadcast_messages(&self, message: Message) {
    let relays = self.relays().await;
    for relay in relays.values() {
      relay.send_message(message.clone());
    }
  }
}

#[derive(Default, Clone, Debug)]
struct AnyCommunicationFromRelay {
  eose: RelayToClientCommEose,
  event: RelayToClientCommEvent,
  notice: RelayToClientCommNotice,
}

#[derive(Default, Debug, Clone)]
struct MsgResult {
  no_op: bool,
  is_eose: bool,
  is_event: bool,
  is_notice: bool,
  data: AnyCommunicationFromRelay,
}
#[derive(Debug, Clone)]
pub struct RelayPoolTask {
  receiver: Arc<Mutex<UnboundedReceiver<RelayPoolMessage>>>,
}

impl RelayPoolTask {
  pub fn new(receiver: UnboundedReceiver<RelayPoolMessage>) -> Self {
    Self {
      receiver: Arc::new(Mutex::new(receiver)),
    }
  }

  /// Helper to parse the function into EOSE, NOTICE or EVENT.
  ///
  fn parse_message_received_from_relay(&self, msg: &str, relay_url: String) -> MsgResult {
    let mut result = MsgResult::default();

    if let Ok(eose_msg) = RelayToClientCommEose::from_json(msg.to_string()) {
      debug!("EOSE from {relay_url}:\n {:?}\n", eose_msg);

      result.is_eose = true;
      result.data.eose = eose_msg;
      return result;
    }

    if let Ok(event_msg) = RelayToClientCommEvent::from_json(msg.to_string()) {
      debug!("EVENT from {relay_url}:\n {:?}\n", event_msg);

      // validates signature
      if !event_msg.event.check_event_signature() {
        result.no_op = true;
        error!("Received an event, but its signature is not valid!");
        debug!("Event signature with error: {:?}", event_msg.event);
        return result;
      }

      result.is_event = true;
      result.data.event = event_msg;
      return result;
    }

    if let Ok(notice_msg) = RelayToClientCommNotice::from_json(msg.to_string()) {
      debug!("NOTICE from {relay_url}:\n {:?}\n", notice_msg);

      result.is_notice = true;
      result.data.notice = notice_msg;
      return result;
    }

    result.no_op = true;
    debug!("NO-OP from {relay_url}: {:?}", msg);
    result
  }

  /// This is responsible for listening (via `receiver`)
  /// for any messages sent to the relay pool via `pool_task_sender`.
  pub async fn run(&mut self) {
    debug!("RelayPool Thread Started");
    while let Some(msg) = self.receiver.lock().await.recv().await {
      match msg {
        RelayPoolMessage::ReceivedMsg { relay_url, msg } => {
          let _ = self.parse_message_received_from_relay(msg.to_text().unwrap(), relay_url);
        }
      }
    }
    debug!("RelayPool Thread Ended");
  }
}

#[cfg(test)]
mod tests {
  use crate::event::Event;

  use super::*;

  #[cfg(test)]
  use pretty_assertions::assert_eq;
  use serde_json::json;

  fn make_relaydata_sut() -> RelayData {
    let (pool_task_sender, _pool_task_receiver) = tokio::sync::mpsc::unbounded_channel();
    RelayData::new(String::from("potato_url"), pool_task_sender)
  }

  fn make_relaypooltask_sut() -> RelayPoolTask {
    let (_pool_task_sender, pool_task_receiver) = tokio::sync::mpsc::unbounded_channel();
    RelayPoolTask::new(pool_task_receiver)
  }

  #[test]
  fn relaydata_disconnect() {
    let relay_data = make_relaydata_sut();

    assert_eq!(relay_data.is_connected.load(Ordering::Relaxed), false);
    assert_eq!(
      relay_data.close_communication.load(Ordering::Relaxed),
      false
    );

    relay_data.disconnect();

    assert_eq!(relay_data.is_connected.load(Ordering::Relaxed), false);
    assert!(relay_data.close_communication.load(Ordering::Relaxed));
  }

  #[tokio::test]
  async fn relaypool_remove_relay() {
    let relay_pool = RelayPool::new();
    let url = String::from("key_potato");
    let relay_data = make_relaydata_sut();

    assert_eq!(relay_pool.relays().await.len(), 0);

    let mut relays = relay_pool.relays_mut().await;
    relays.insert(url.clone(), relay_data);
    drop(relays);

    assert_eq!(relay_pool.relays().await.len(), 1);
    // if the key does not exist, should not do anything
    relay_pool
      .remove_relay(String::from("non-existent url"))
      .await;
    assert_eq!(relay_pool.relays().await.len(), 1);

    // act
    relay_pool.remove_relay(url.clone()).await;
    assert_eq!(relay_pool.relays().await.len(), 0);
  }

  #[tokio::test]
  async fn relaypool_disconnect_relay() {
    let relay_pool = RelayPool::new();
    let url = String::from("key_potato");
    let relay_data = make_relaydata_sut();

    assert_eq!(relay_pool.relays().await.len(), 0);

    let mut relays = relay_pool.relays_mut().await;
    relays.insert(url.clone(), relay_data);
    drop(relays);
    assert_eq!(relay_pool.relays().await.len(), 1);

    let relays = relay_pool.relays().await;

    // if the key does not exist, should not do anything
    relay_pool
      .disconnect_relay(String::from("non-existent url"))
      .await;
    assert_eq!(relay_pool.relays().await.len(), 1);
    assert_eq!(relays[&url].is_connected.load(Ordering::Relaxed), false);
    assert_eq!(
      relays[&url].close_communication.load(Ordering::Relaxed),
      false
    );

    // act
    relay_pool.disconnect_relay(url.clone()).await;
    assert_eq!(relay_pool.relays().await.len(), 1);

    assert_eq!(relays[&url].is_connected.load(Ordering::Relaxed), false);
    assert!(relays[&url].close_communication.load(Ordering::Relaxed));
  }

  #[test]
  fn parse_eose_message() {
    let relay_pool_task = make_relaypooltask_sut();
    let eose = RelayToClientCommEose::default();
    let eose_json = eose.as_json();

    let result =
      relay_pool_task.parse_message_received_from_relay(&eose_json, String::from("potato_url"));

    assert_eq!(result.data.eose, eose);
    assert!(result.is_eose);
    assert_eq!(result.is_event, false);
    assert_eq!(result.is_notice, false);
    assert_eq!(result.no_op, false);
  }

  #[test]
  fn parse_notice_message() {
    let relay_pool_task = make_relaypooltask_sut();
    let notice = RelayToClientCommNotice::default();
    let notice_json = notice.as_json();

    let result =
      relay_pool_task.parse_message_received_from_relay(&notice_json, String::from("potato_url"));

    assert_eq!(result.data.notice, notice);
    assert!(result.is_notice);
    assert_eq!(result.is_event, false);
    assert_eq!(result.is_eose, false);
    assert_eq!(result.no_op, false);
  }

  #[test]
  fn parse_event_message() {
    let relay_pool_task = make_relaypooltask_sut();
    let event_with_correct_signature = Event::from_value(
      json!({"content":"potato","created_at":1684589418,"id":"00960bd35499f8c63a4f65e79d6b1a2b7f1b8c97e76652325567b78c496350ae","kind":1,"pubkey":"614a695bab54e8dc98946abdb8ec019599ece6dada0c23890977d0fa128081d6","sig":"bf073c935f71de50ec72bdb79f75b0bf32f9049305c3b22f97c06422c6f2edc86e0d7e07d7d7222678b238b1daee071be5f6fa653c611971395ec0d1c6407caf","tags":[]}),
    ).unwrap();
    let event =
      RelayToClientCommEvent::new_event(String::from("potato_subs"), event_with_correct_signature);
    let event_json = event.as_json();

    let result =
      relay_pool_task.parse_message_received_from_relay(&event_json, String::from("potato_url"));

    assert_eq!(result.data.event, event);
    assert!(result.is_event);
    assert_eq!(result.is_notice, false);
    assert_eq!(result.is_eose, false);
    assert_eq!(result.no_op, false);
  }

  #[test]
  fn parse_noop_message() {
    let relay_pool_task = make_relaypooltask_sut();
    let no_op = r#"{}"#;

    let result =
      relay_pool_task.parse_message_received_from_relay(no_op, String::from("potato_url"));

    assert!(result.no_op);
    assert_eq!(result.is_notice, false);
    assert_eq!(result.is_eose, false);
    assert_eq!(result.is_event, false);
  }
}
