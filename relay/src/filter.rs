use std::collections::HashMap;

use serde::{Deserialize, Serialize};

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
/// - tags: list of tags
///   [
///     e: a list of event ids that are referenced in an "e" tag,
///     p: a list of pubkeys that are referenced in an "p" tag,
///     ...
///   ]
/// - since: a timestamp. Events must be newer than this to pass
/// - until: a timestamp. Events must be older than this to pass
/// - limit: maximum number of events to be returned in the initial query (it can be ignored afterwards)
///
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Filter {
  ids: Option<Vec<String>>,
  authors: Option<Vec<String>>,
  kinds: Option<Vec<u64>>,
  tags: Option<HashMap<String, Vec<String>>>,
  since: Option<String>,
  until: Option<String>,
  limit: Option<u64>,
}