use env_logger::Env;
use relay::relay;

fn main() {
  env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
  relay::initiate_relay().expect("Error while trying to instatiante relay WS");
}
