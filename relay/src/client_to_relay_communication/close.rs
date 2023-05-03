use std::{net::SocketAddr, sync::MutexGuard};

use crate::relay::ClientConnectionInfo;

use super::types::ClientToRelayCommClose;

pub fn on_close_message(
  client_close: ClientToRelayCommClose,
  clients: &mut MutexGuard<Vec<ClientConnectionInfo>>,
  addr: SocketAddr,
) {
  match clients.iter().position(|client| client.socket_addr == addr) {
    Some(client_idx) => {
      // Client can only close the subscription of its own connection
      match clients[client_idx]
        .requests
        .iter()
        .position(|client_req| client_req.subscription_id == client_close.subscription_id)
      {
        Some(client_req_index) => {
          clients[client_idx].requests.remove(client_req_index);
        }
        None => (),
      }
    }
    None => (),
  };
}
