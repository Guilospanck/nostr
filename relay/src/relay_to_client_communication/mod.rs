use tokio_tungstenite::tungstenite::Message;

use crate::relay::Tx;

// internal modules
pub mod types;
pub mod event;
pub mod eose;

#[derive(Debug)]
pub struct OutboundInfo {
  pub tx: Tx,
  pub content: String,
}

pub fn send_message_to_client(tx: Tx, content: String) {
  tx.unbounded_send(Message::binary(format!("{}", content).as_bytes()))
    .unwrap();
}

pub fn broadcast_message_to_clients(outbound_client_and_message: Vec<OutboundInfo>) {
  for recp in outbound_client_and_message {
    send_message_to_client(recp.tx.clone(), recp.content.clone());
  }
}