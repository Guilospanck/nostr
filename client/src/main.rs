use env_logger::Env;
use client::client;

fn main() {
  std::env::set_var("RUST_LOG_STYLE", "always");
  env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();
  client::initiate_client().expect("Could not start client");
}