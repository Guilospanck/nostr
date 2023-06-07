use client::client;
use env_logger::Env;

#[tokio::main]
async fn main() {
  dotenv::dotenv().ok();
  env_logger::Builder::from_env(Env::default().default_filter_or("debug"))
    .try_init()
    .unwrap();
  let client = client::Client::new();
  client.connect().await;
}
