use env_logger::Env;
use relay::relay;

fn main() {
  std::env::set_var("RUST_LOG_STYLE", "always");
  env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();
  relay::initiate_relay().expect("Error while trying to instantiate relay WS");
}
