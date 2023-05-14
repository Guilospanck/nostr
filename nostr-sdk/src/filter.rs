use serde::{Deserialize, Serialize};

use crate::event::{id::EventId, kind::EventKind, PubKey, Timestamp};

///
/// Filters are data structures that clients send to relays (being the first on the first connection)
/// to request data from other clients.
/// The attributes of a Filter work as `&&` (in other words, all the conditions set must be present
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
  #[serde(alias = "#e")]
  pub e: Option<Vec<String>>,
  #[serde(alias = "#p")]
  pub p: Option<Vec<String>>,
  pub since: Option<Timestamp>,
  pub until: Option<Timestamp>,
  pub limit: Option<Timestamp>,
}

impl Filter {
  pub fn as_str(&self) -> String {
    serde_json::to_string(self).unwrap()
  }

  pub fn from_string(data: String) -> Result<Self, serde_json::error::Error> {
    serde_json::from_str(&data)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[cfg(test)]
  use pretty_assertions::assert_eq;
  use serde_json::{json, Value};

  #[test]
  fn from_string() {
    let filter = json!(
    {
      "e": [
        "44b17a5acd66694cbdf5aea08968453658446368d978a15e61e599b8404d82c4",
        "7742783afbf6b283e81af63782ab0c05bbcbccba7f3abce0e0f23706dc27bd42",
        "9621051bcd8723f03da00aae61ee46956936726fcdfa6f34e29ae8f1e2b63cb5"
      ],
      "#p": ["potato"],
      "kinds": [1, 6, 7, 9735]
    })
    .to_string();

    let filter2 = json!(
    {
      "#e": [
        "44b17a5acd66694cbdf5aea08968453658446368d978a15e61e599b8404d82c4",
        "7742783afbf6b283e81af63782ab0c05bbcbccba7f3abce0e0f23706dc27bd42",
        "9621051bcd8723f03da00aae61ee46956936726fcdfa6f34e29ae8f1e2b63cb5"
      ],
      "p": ["potato"],
      "kinds": [1, 6, 7, 9735]
    })
    .to_string();

    let filter3 = "{\"#e\":[\"44b17a5acd66694cbdf5aea08968453658446368d978a15e61e599b8404d82c4\",\"7742783afbf6b283e81af63782ab0c05bbcbccba7f3abce0e0f23706dc27bd42\",\"9621051bcd8723f03da00aae61ee46956936726fcdfa6f34e29ae8f1e2b63cb5\"],\"#p\":[\"potato\"],\"kinds\":[1,6,7,9735]}".to_string();

    let result = Filter::from_string(filter).unwrap();
    let result2 = Filter::from_string(filter2).unwrap();
    let result3 = Filter::from_string(filter3).unwrap();
    let expected = Filter {
      e: Some(vec![
        "44b17a5acd66694cbdf5aea08968453658446368d978a15e61e599b8404d82c4".to_string(),
        "7742783afbf6b283e81af63782ab0c05bbcbccba7f3abce0e0f23706dc27bd42".to_string(),
        "9621051bcd8723f03da00aae61ee46956936726fcdfa6f34e29ae8f1e2b63cb5".to_string(),
      ]),
      p: Some(vec!["potato".to_string()]),
      kinds: Some(vec![
        EventKind::Text,
        EventKind::Custom(6),
        EventKind::Custom(7),
        EventKind::Custom(9735),
      ]),
      ..Default::default()
    };

    assert_eq!(result, expected);
    assert_eq!(result2, expected);
    assert_eq!(result3, expected);
  }

  #[test]
  fn as_str() {
    let filter = Filter {
      e: Some(vec![
        "44b17a5acd66694cbdf5aea08968453658446368d978a15e61e599b8404d82c4".to_string(),
        "7742783afbf6b283e81af63782ab0c05bbcbccba7f3abce0e0f23706dc27bd42".to_string(),
        "9621051bcd8723f03da00aae61ee46956936726fcdfa6f34e29ae8f1e2b63cb5".to_string(),
      ]),
      p: Some(vec!["potato".to_string()]),
      kinds: Some(vec![
        EventKind::Text,
        EventKind::Custom(6),
        EventKind::Custom(7),
        EventKind::Custom(9735),
      ]),
      ..Default::default()
    };

    let expected = json!(
    {
      "ids":null,
      "authors":null,
      "kinds":[1,6,7,9735],
      "e":[
        "44b17a5acd66694cbdf5aea08968453658446368d978a15e61e599b8404d82c4",
        "7742783afbf6b283e81af63782ab0c05bbcbccba7f3abce0e0f23706dc27bd42",
        "9621051bcd8723f03da00aae61ee46956936726fcdfa6f34e29ae8f1e2b63cb5"
        ],
        "#p":["potato"],
        "since":null,
        "until":null,
        "limit":null
    });

    let result = filter.as_str();
    let result: Value = serde_json::from_str(&result).unwrap();

    assert_eq!(result["kinds"], expected["kinds"]);
    assert_eq!(result["e"], expected["e"]);
    assert_eq!(result["p"], expected["#p"]);
    assert_eq!(result["authors"], expected["authors"]);
  }
}
