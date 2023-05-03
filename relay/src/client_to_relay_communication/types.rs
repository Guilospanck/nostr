use serde::{Deserialize, Serialize};

use crate::event::Event;
use crate::filter::Filter;

/// The three types of `client -> relay` communications.
/// 
///  - `["EVENT", event_JSON]`: used to publish events
/// 
///  - `["REQ", subscription_id, filters_JSON]`: used to request events and subscribe to new updates.
///       A REQ message may contain multiple filters. In this case, events that match any of the filters are to be returned,
///       i.e., multiple filters are to be interpreted as `||` conditions.
///
///  - `["CLOSE", subscription_id]`: used to stop previous subscriptions. `subscription_id` is a random string used to represent a subscription.
///
pub enum ClientToRelayComm {
  Event,
  Request,
  Close,
}

impl ClientToRelayComm {
  fn as_str(&self) -> &'static str {
    match self {
      ClientToRelayComm::Event => "EVENT",
      ClientToRelayComm::Request => "REQ",
      ClientToRelayComm::Close => "CLOSE",
    }
  }
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct ClientToRelayCommEvent {
  pub code: String, // "EVENT"
  pub event: Event,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct ClientToRelayCommRequest {
  pub code: String, // "REQ"
  pub subscription_id: String,
  pub filters: Vec<Filter>,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct ClientToRelayCommClose {
  pub code: String, // "CLOSE"
  pub subscription_id: String,
}