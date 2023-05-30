use env_logger::Env;
use client::client;

fn main() {
  dotenv::dotenv().ok();
  env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();
  let client = client::Client::new();
  client.connect();
}