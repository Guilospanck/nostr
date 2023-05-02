use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::event::{kind::EventKind, PubKey, id::EventId, Timestamp};

///
/// Filters are data structures that clients send to relays (being the first on the first connection)
/// to request data from other clients.
/// The attributes of a Filter work as && (in other words, all the conditions set must be present
/// in the event in order to pass the filter).
/// P.S.: a "REQ" communication from the client can have multiple filters. In this case, all filters will be
/// used as `||` operator: anything that matches any of the filters will be sent.
///
/// - ids: a list of events of prefixes
/// - authors: a list of publickeys or prefixes, the pubkey of an event must be one of these
/// - kinds: a list of kind numbers
/// - e: a list of event ids that are referenced in an "e" tag,
/// - p: a list of pubkeys that are referenced in an "p" tag,
/// - since: a timestamp. Events must be newer than this to pass
/// - until: a timestamp. Events must be older than this to pass
/// - limit: maximum number of events to be returned in the initial query (it can be ignored afterwards)
///
#[derive(Debug, Serialize, Deserialize, Default, Clone, PartialEq, Eq)]
pub struct Filter {
  pub ids: Option<Vec<EventId>>,
  pub authors: Option<Vec<PubKey>>,
  pub kinds: Option<Vec<EventKind>>,
  pub tags: Option<HashMap<String, Vec<String>>>,
  #[serde(rename = "#e")]
  pub e: Option<Vec<String>>,
  #[serde(rename = "#p")]
  pub p: Option<Vec<String>>,
  pub since: Option<Timestamp>,
  pub until: Option<Timestamp>,
  pub limit: Option<Timestamp>,
}
