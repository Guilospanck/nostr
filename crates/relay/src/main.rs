use env_logger::Env;
use guilospanck_nostr_sdk::relay;

fn main() {
  dotenv::dotenv().ok();
  env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
  relay::initiate_relay().expect("Error while trying to instantiate relay WS");
}
