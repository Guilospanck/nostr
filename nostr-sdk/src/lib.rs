pub use env_logger::Env;
pub use log::{debug, info};

use std::sync::Once;

static INIT_LOGGER: Once = Once::new();

fn init_logger() {
  INIT_LOGGER.call_once(|| {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
  });
}

pub mod client_to_relay_communication;
pub mod event;
pub mod filter;
pub mod relay_to_client_communication;
pub mod schnorr;
