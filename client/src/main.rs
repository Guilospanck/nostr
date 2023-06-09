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
  client.follow_author(String::from("764595da089dd12cca1d1c2fa917a212b249a40e95fb2ac39e3a131a7d7fab52")).await;
  client
    .name("Nostr Client")
    .about("This is a nostr client")
    .picture("someurl.image.com")
    .send_updated_metadata().await;

  let ctrl_c = async {
    tokio::signal::ctrl_c().await.unwrap();
  };
  join!(ctrl_c);
}
