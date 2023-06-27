use log::debug;
use tokio_tungstenite::tungstenite::Message;

use crate::relay::Tx;

#[derive(Debug, Clone)]
pub struct OutboundInfo {
  pub tx: Tx,
  pub content: String,
}

pub fn send_message_to_client(tx: Tx, content: String) {
  debug!("{content}");
  tx.send(Message::Text(content)).unwrap();
}

pub fn broadcast_message_to_clients(outbound_client_and_message: Vec<OutboundInfo>) {
  for recp in outbound_client_and_message {
    send_message_to_client(recp.tx.clone(), recp.content.clone());
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[cfg(test)]
  use pretty_assertions::assert_eq;
  use tokio::sync::mpsc::UnboundedReceiver;

  struct Sut {
    outbound_info: OutboundInfo,
    rx: UnboundedReceiver<Message>,
  }

  fn make_sut(content: &str) -> Sut {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<Message>();

    Sut {
      outbound_info: OutboundInfo { tx, content: content.to_string() },
      rx,
    }
  }

  #[tokio::test]
  async fn test_send_message_to_client() {
    let mut sut = make_sut("first_content");

    send_message_to_client(sut.outbound_info.tx, sut.outbound_info.content.clone());

    let received = sut.rx.recv().await.unwrap();
    assert_eq!(received.to_string(), sut.outbound_info.content);
  }

  #[tokio::test]
  async fn test_broadcast_message_to_clients() {
    let mut sut1 = make_sut("first_content");
    let mut sut2 = make_sut("second_content");

    broadcast_message_to_clients(vec![sut1.outbound_info.clone(), sut2.outbound_info.clone()]);

    let received1 = sut1.rx.recv().await.unwrap();
    let received2 = sut2.rx.recv().await.unwrap();
    assert_eq!(received1.to_string(), sut1.outbound_info.content);
    assert_eq!(received2.to_string(), sut2.outbound_info.content);
  }
}
