use std::{collections::HashMap, sync::Arc};

use futures_util::SinkExt;
use futures_util::StreamExt;
use log::debug;
use log::error;
use log::info;
use tokio::join;
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
  // TODO: this will be used when we add new relays to the pool.
  pool_task_sender: PoolTaskSender,
  relay_pool_task: RelayPoolTask
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
      relay_pool_task
    }
  }

  pub async fn relays(&self) -> HashMap<String, RelayData> {
    let relays = self.relays.lock().await;
    relays.clone()
  }

  pub async fn connect(&self, metadata: Message) {
    let relays = self.relays().await;

    let mut relay_pool_task = self.relay_pool_task.clone();
    let listener = tokio::spawn(async move {
      relay_pool_task.run().await;
    });

    for relay in relays.values() {
      relay.connect(metadata.clone()).await;
    }

    let _ = join!(listener);
  }

  pub async fn broadcast_messages(&self, message: Message) {
    let relays = self.relays().await;

    for relay in relays.values() {
      relay.send_message(message.clone());
    }
  }

  pub async fn send_message_to_relay(&self, url: String, message: Message) {
    let relays = self.relays().await;
    match relays.get(&url) {
      Some(relay) => relay.relay_tx.send(message).unwrap(),
      None => {
        error!("Relay does not exist in the pool")
      }
    }
  }
}

#[derive(Debug, Clone)]
pub struct RelayPoolTask {
  receiver: Arc<Mutex<UnboundedReceiver<RelayPoolMessage>>>,
}

impl RelayPoolTask {
  pub fn new(receiver: UnboundedReceiver<RelayPoolMessage>) -> Self {
    Self { receiver: Arc::new(Mutex::new(receiver)) }
  }

  /// This is responsible for listening (via `receiver`)
  /// for any messages sent to the relay pool via `pool_task_sender`.
  pub async fn run(&mut self) {
    debug!("RelayPool Thread Started");
    while let Some(msg) = self.receiver.lock().await.recv().await {
      match msg {
        RelayPoolMessage::ReceivedMsg { relay_url, msg } => {
          debug!(
            "Received message from relay {}: {}",
            relay_url,
            msg.to_text().unwrap()
          );
        }
      }
    }
    debug!("RelayPool Thread Ended");
  }
}
