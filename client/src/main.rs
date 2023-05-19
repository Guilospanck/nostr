use env_logger::Env;
use client::client;

fn main() {
  env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
  client::initiate_client().expect("Could not start client");
}