use log::debug;
use tokio_tungstenite::tungstenite::Message;

use crate::relay::Tx;

#[derive(Debug)]
pub struct OutboundInfo {
  pub tx: Tx,
  pub content: String,
}

pub fn send_message_to_client(tx: Tx, content: String) {
  debug!("===============================================================");
  debug!("Sending message to client:");
  debug!("{content}");
  debug!("===============================================================");
  tx.unbounded_send(Message::Text(content))
    .unwrap();
}

pub fn broadcast_message_to_clients(outbound_client_and_message: Vec<OutboundInfo>) {
  for recp in outbound_client_and_message {
    send_message_to_client(recp.tx.clone(), recp.content.clone());
  }
}
