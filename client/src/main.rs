use client::client;
use env_logger::Env;
use futures_util::join;

#[tokio::main]
async fn main() {
  dotenv::dotenv().ok();
  env_logger::Builder::from_env(Env::default().default_filter_or("debug"))
    .try_init()
    .unwrap();
  let mut client = client::Client::new();
  client.connect().await;
  client.get_notifications().await;
  // client.follow_author(String::from("82341f882b6eabcd2ba7f1ef90aad961cf074af15b9ef44a09f9d2a8fbfbe6a2")).await; // jack's pubkey
  client
    .name("Nostr Client")
    .about("This is a nostr client")
    .picture("someurl.image.com")
    .send_updated_metadata().await;
  client.add_relay(String::from("wss://relay.damus.io")).await;
  client.subscribe_to_all_stored_requests().await;

  let ctrl_c = async {
    tokio::signal::ctrl_c().await.unwrap();
  };
  join!(ctrl_c);
}
