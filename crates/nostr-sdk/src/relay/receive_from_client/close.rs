use std::{net::SocketAddr, sync::MutexGuard};

use crate::relay::ClientConnectionInfo;

pub fn on_close_message(
  subscription_id: String,
  clients: &mut MutexGuard<Vec<ClientConnectionInfo>>,
  addr: SocketAddr,
) -> bool {
  match clients.iter().position(|client| client.socket_addr == addr) {
    Some(client_idx) => {
      // Client can only close the subscription of its own connection
      match clients[client_idx]
        .requests
        .iter()
        .position(|client_req| client_req.subscription_id == subscription_id)
      {
        Some(client_req_index) => {
          clients[client_idx].requests.remove(client_req_index);
          true
        }
        None => false,
      }
    }
    None => false,
  }
}

#[cfg(test)]
mod tests {
  use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{Arc, Mutex},
  };

  use crate::filter::Filter;

  use crate::relay::{ClientRequests, Tx};

  use super::*;

  #[cfg(test)]
  use pretty_assertions::assert_eq;
  use tokio_tungstenite::tungstenite::Message;

  struct CloseSut {
    mock_clients: Arc<Mutex<Vec<ClientConnectionInfo>>>,
    mock_addr: SocketAddr,
    mock_tx: Tx,
    mock_subscription_id: String,
  }

  impl CloseSut {
    fn new() -> Self {
      let mock_clients: Arc<Mutex<Vec<ClientConnectionInfo>>> =
        Arc::new(Mutex::new(Vec::<ClientConnectionInfo>::new()));

      let mock_subscription_id = "mock_subscription_id".to_string();

      let mock_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);

      let (mock_tx, _rx) = tokio::sync::mpsc::unbounded_channel::<Message>();

      Self {
        mock_addr,
        mock_clients,
        mock_tx,
        mock_subscription_id,
      }
    }
  }

  #[test]
  fn test_on_close_message_should_do_nothing_when_socket_addresses_are_not_equal() {
    let mock = CloseSut::new();
    let mut clients = mock.mock_clients.lock().unwrap();
    clients.push(ClientConnectionInfo {
      tx: mock.mock_tx.clone(),
      socket_addr: mock.mock_addr,
      requests: vec![ClientRequests {
        subscription_id: mock.mock_subscription_id.clone(),
        filters: vec![Filter::default()],
      }],
    });
    let another_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8081);

    on_close_message(mock.mock_subscription_id, &mut clients, another_addr);

    assert_eq!(clients.len(), 1);
    assert_eq!(clients[0].requests.len(), 1);
  }

  #[test]
  fn test_on_close_message_should_do_nothing_when_subscription_ids_are_not_equal() {
    let mock = CloseSut::new();
    let mut clients = mock.mock_clients.lock().unwrap();
    clients.push(ClientConnectionInfo {
      tx: mock.mock_tx.clone(),
      socket_addr: mock.mock_addr,
      requests: vec![ClientRequests {
        subscription_id: "another_subs_id".to_string(),
        filters: vec![Filter::default()],
      }],
    });

    on_close_message(mock.mock_subscription_id, &mut clients, mock.mock_addr);

    assert_eq!(clients.len(), 1);
    assert_eq!(clients[0].requests.len(), 1);
  }

  #[test]
  fn test_on_close_message_should_remove_client_reqs() {
    let mock = CloseSut::new();
    let mut clients = mock.mock_clients.lock().unwrap();
    clients.push(ClientConnectionInfo {
      tx: mock.mock_tx.clone(),
      socket_addr: mock.mock_addr,
      requests: vec![ClientRequests {
        subscription_id: mock.mock_subscription_id.clone(),
        filters: vec![Filter::default()],
      }],
    });

    on_close_message(mock.mock_subscription_id, &mut clients, mock.mock_addr);

    assert_eq!(clients.len(), 1);
    assert_eq!(clients[0].requests.len(), 0);
  }
}
