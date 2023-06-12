use std::{collections::HashMap, sync::Arc};

use futures_util::SinkExt;
use futures_util::StreamExt;
use log::debug;
use log::error;
use log::info;
use nostr_sdk::relay_to_client_communication::eose::RelayToClientCommEose;
use nostr_sdk::relay_to_client_communication::event::RelayToClientCommEvent;
use nostr_sdk::relay_to_client_communication::notice::RelayToClientCommNotice;
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
}

impl RelayData {
  pub fn new(url: String, pool_task_sender: PoolTaskSender) -> Self {
    let (relay_tx, relay_rx) = unbounded_channel();

    Self {
      url,
      pool_task_sender,
      relay_tx,
      relay_rx: Arc::new(Mutex::new(relay_rx)),
    }
  }

  async fn connect(&self, metadata: Message) {
    debug!("Connecting to {}", self.url.clone());

    let connection = connect_async(self.url.clone()).await;

    // Connect
    match connection {
      Ok((ws_stream, _)) => {
        info!("Connected to {}", self.url.clone());
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
          debug!("Relay Message Thread Started");

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

          debug!("Exited from Message Thread of {}", relay.url);
        });

        // Send messages sent to this relay, which were sent by our client.
        let relay = self.clone();
        tokio::spawn(async move {
          let mut rx = relay.relay_rx.lock().await;
          while let Some(msg) = rx.recv().await {
            let _ = ws_tx.send(msg).await;
          }
        });
      }
      Err(err) => {
        error!("Impossible to connect to {}: {}", self.url, err);
      }
    };
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

impl RelayPool {
  pub fn new() -> Self {
    // create channel to allow relays to communicate with the pool
    let (pool_task_sender, pool_task_receiver) = tokio::sync::mpsc::unbounded_channel();

    // creates the pool task in order to handle messages sent to it
    let relay_pool_task = RelayPoolTask::new(pool_task_receiver);

    // get initial relay
    let relay_url = String::from("ws://127.0.0.1:8080/");
    let relay = RelayData::new(relay_url.clone(), pool_task_sender.clone());
    let mut relays = HashMap::new();
    relays.insert(relay_url, relay);
    let relays = Arc::new(Mutex::new(relays));

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

  pub async fn add_relay(&self, url: String, metadata: Message) {
    let mut relays = self.relays_mut().await;

    if relays.get(&url).is_none() {
      let relay = RelayData::new(url.clone(), self.pool_task_sender.clone());
      relays.insert(url, relay.clone());
      relay.connect(metadata).await;
    }
  }

  pub async fn remove_relay(&self, url: String) {
    let mut relays = self.relays_mut().await;
    relays.remove(&url);
  }

  pub async fn connect(&self, metadata: Message) {
    let relays = self.relays().await;

    for relay in relays.values() {
      relay.connect(metadata.clone()).await;
    }
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
    debug!("NO-OP from {relay_url}: {:?}\n", msg);
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
