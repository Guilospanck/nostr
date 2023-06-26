use env_logger::Env;
use futures_util::join;

use nostr_sdk::client;

#[tokio::main]
async fn main() {
  dotenv::dotenv().ok();
  env_logger::Builder::from_env(Env::default().default_filter_or("debug"))
    .try_init()
    .unwrap();

  let mut client = client::Client::new();

  client.connect().await;
  client.get_notifications().await;
  client
    .follow_author(String::from(
      "82341f882b6eabcd2ba7f1ef90aad961cf074af15b9ef44a09f9d2a8fbfbe6a2",
    ))
    .await; // jack's pubkey
            // client.follow_myself().await;
  client
    .follow_author(String::from(
      "5081ce98f7da142513444079a55e2d1676559a908d4f694d299057f8abddf835",
    ))
    .await;
  client
    .name("Nostr Client")
    .about("This is a nostr client")
    .picture("someurl.image.com")
    .send_updated_metadata()
    .await;
  // client.add_relay(String::from("wss://relay.damus.io")).await;
  // client.add_relay(String::from("wss://nostr.wine")).await;
  // client
  //   .add_relay(String::from("wss://pow.nostrati.com"))
  //   .await;
  // client.subscribe_to_all_stored_requests().await;
  // client.unsubscribe("d8e67092-c17f-4934-8b7d-6c97cb697cc1").await;
  // client.publish_text_note("TESTING!!!".to_string()).await;

  // reply to root
  // let event = client.create_event(EventKind::Text, "REPLY TO THIS IF YOU CAN".to_string(), None);
  // let content = String::from("Replying 😎");
  // let marker = Marker::Root;
  // client.reply_to_event(event, None, marker, content).await;

  // reply to reply
  // let content = String::from("Replying to reply! 😎");
  // let marker = Marker::Reply;
  // let event = json!({
  //   "content": "Replying 😎",
  //   "created_at": 1686668598,
  //   "id": "d082deb5083de8f9a3607c5c7891454332c1376825dfedb05517d7bb053b8695",
  //   "kind": 1,
  //   "pubkey": "2c3e48a0146aa2831ffa4b7cf09a5dec58c597f8111cb8063938cbacb2ac808d",
  //   "sig": "ac7800f1ec735b88ec45e678cf6fa6f3dde338fda4dfdc4a53c717b1cd28297f2559b08943e4e2e1c0860f0a7948ea0cf7bb3b04623dafcf36db80d4a3645ce3",
  //   "tags": [
  //     [
  //       "e",
  //       "2816166989f3cb75ff837b3d352b02a3b0147587807421488182cf3d37af0ca3",
  //       "",
  //       "root"
  //     ],
  //     [
  //       "p",
  //       "82341f882b6eabcd2ba7f1ef90aad961cf074af15b9ef44a09f9d2a8fbfbe6a2"
  //     ]
  //   ]
  // });
  // let event = Event::from_value(event).unwrap();
  // client.reply_to_event(event, None, marker, content).await;

  //
  // sleep(Duration::new(19, 0));

  // client
  //   .close_connection(String::from("ws://127.0.0.1:8080/"))
  //   .await;

  let ctrl_c = async {
    tokio::signal::ctrl_c().await.unwrap();
  };
  join!(ctrl_c);
}
