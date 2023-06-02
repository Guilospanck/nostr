use std::sync::Arc;

use env_logger::Env;
use client::client;

fn main() {
  dotenv::dotenv().ok();
  env_logger::Builder::from_env(Env::default().default_filter_or("debug")).try_init().unwrap();
  let client = Arc::new(client::Client::new());
  client.clone().connect();
  client.notifications();
}