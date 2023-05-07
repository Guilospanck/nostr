use serde::{ser::SerializeSeq, Deserialize, Deserializer, Serialize, Serializer};

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

#[derive(Debug, Clone)]
pub struct ClientToRelayCommRequest {
  pub code: String, // "REQ"
  pub subscription_id: String,
  pub filters: Vec<Filter>,
}

#[derive(Debug, Clone)]
pub struct ClientToRelayCommClose {
  pub code: String, // "CLOSE"
  pub subscription_id: String,
}
