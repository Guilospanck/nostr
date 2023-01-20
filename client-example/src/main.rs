//! A simple example of hooking up stdin/stdout to a WebSocket stream.
//!
//! This example will connect to a server specified in the argument list and
//! then forward all data read on stdin to the server, printing out all data
//! received on stdout.
//!
//! Note that this is not currently optimized for performance, especially around
//! buffer management. Rather it's intended to show an example of working with a
//! client.
//!
//! You can use this example together with the `server` example.

use std::{collections::HashMap, env, sync::{Arc, Mutex}};

use futures_util::{future, pin_mut, StreamExt};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Default, Deserialize)]
pub struct Filter {
  pub ids: Option<Vec<String>>,
  pub authors: Option<Vec<String>>,
  pub kinds: Option<Vec<u64>>,
  pub tags: Option<HashMap<String, Vec<String>>>,
  pub since: Option<String>,
  pub until: Option<String>,
  pub limit: Option<u64>,
}

pub const LIST_OF_RELAYS: [&str; 1] = ["ws://127.0.0.1:8080/"];

#[tokio::main]
async fn main() {
  let subscriptions_ids = Arc::new(Mutex::new(Vec::<String>::new()));

  for addr in LIST_OF_RELAYS.iter() {
    println!("Connecting to relay {:?}...", addr);
    tokio::spawn(handle_connection(
      *addr.to_string(),
      subscriptions_ids.clone(),
    ));
  }

  
}

pub async fn handle_connection(connect_addr: String, subscriptions_ids: Arc<Mutex<Vec<Vec<String>>>>) {
  let url = url::Url::parse(&connect_addr).unwrap();

  let (stdin_tx, stdin_rx) = futures_channel::mpsc::unbounded();
  tokio::spawn(read_stdin(stdin_tx.clone()));

  let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
  println!("WebSocket handshake has been successfully completed");

  // send initial message
  send_initial_message(stdin_tx, &mut subscriptions_ids).await;

  let (write, read) = ws_stream.split();

  let stdin_to_ws = stdin_rx.map(Ok).forward(write);

  // This will print to stdout whatever the WS sends
  // (The WS is forwarding messages from other clients)
  let ws_to_stdout = {
    read.for_each(|message| async {
      match message {
        Ok(msg) => {
          let data = msg.into_data();
          tokio::io::stdout().write_all(&data).await.unwrap();
        }
        Err(_err) => {
          eprintln!("Connection closed");
          return ();
        }
      }
    })
  };

  pin_mut!(stdin_to_ws, ws_to_stdout);
  future::select(stdin_to_ws, ws_to_stdout).await;
}

// Our helper method which will read data from stdin and send it along the
// sender provided.
// Send STDIN to WS
async fn read_stdin(tx: futures_channel::mpsc::UnboundedSender<Message>) {
  let mut stdin = tokio::io::stdin();
  loop {
    let mut buf = vec![0; 1024];
    let n = match stdin.read(&mut buf).await {
      Err(_) | Ok(0) => break,
      Ok(n) => n,
    };
    buf.truncate(n);
    tx.unbounded_send(Message::binary(buf)).unwrap();
  }
}

// Our helper method which will send initial data upon connection.
// It will require some data from the relay using a filter subscription.
async fn send_initial_message(
  tx: futures_channel::mpsc::UnboundedSender<Message>,
  subscriptions_ids: &mut Vec<String>,
) {
  let filter = Filter {
    ids: Some(
      ["ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb".to_owned()].to_vec(),
    ),
    authors: None,
    kinds: None,
    tags: None,
    since: None,
    until: None,
    limit: None,
  };

  let filter_string = serde_json::to_string(&filter).unwrap();
  let subscription_id = Uuid::new_v4().to_string();
  subscriptions_ids.push(subscription_id.clone());

  // ["REQ","some-random-subs-id",{"ids":["ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb"],"authors":null,"kinds":null,"tags":null,"since":null,"until":null,"limit":null}]
  // ["EVENT",{"id":"ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb","pubkey":"02c7e1b1e9c175ab2d100baf1d5a66e73ecc044e9f8093d0c965741f26aa3abf76","created_at":1673002822,"kind":1,"tags":[["e","688787d8ff144c502c7f5cffaafe2cc588d86079f9de88304c26b0cb99ce91c6","wss://relay.damus.io"],["p","02c7e1b1e9c175ab2d100baf1d5a66e73ecc044e9f8093d0c965741f26aa3abf76",""]],"content":"Lorem ipsum dolor sit amet","sig":"e8551d85f530113366e8da481354c2756605e3f58149cedc1fb9385d35251712b954af8ef891cb0467d50ddc6685063d4190c97e9e131f903e6e4176dc13ce7c"}]
  // ["CLOSE","some-random-subs-id"]
  let filter_subscription = format!(
    "[\"{}\",\"{}\",{}]",
    String::from("REQ"),
    subscription_id,
    filter_string
  );

  tx.unbounded_send(Message::binary(filter_subscription.as_bytes()))
    .unwrap();
}
