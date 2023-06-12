use std::vec;

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
  #[serde(skip_serializing_if="Option::is_none")]
  pub ids: Option<Vec<EventId>>,
  #[serde(skip_serializing_if="Option::is_none")]
  pub authors: Option<Vec<PubKey>>,
  #[serde(skip_serializing_if="Option::is_none")]
  pub kinds: Option<Vec<EventKind>>,
  #[serde(alias = "#e", rename(serialize = "#e"), skip_serializing_if="Option::is_none")]
  pub e: Option<Vec<String>>,
  #[serde(alias = "#p", rename(serialize = "#p"), skip_serializing_if="Option::is_none")]
  pub p: Option<Vec<String>>,
  #[serde(skip_serializing_if="Option::is_none")]
  pub since: Option<Timestamp>,
  #[serde(skip_serializing_if="Option::is_none")]
  pub until: Option<Timestamp>,
  #[serde(skip_serializing_if="Option::is_none")]
  pub limit: Option<Timestamp>,
}

impl Filter {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn add_ids(&mut self, ids: Vec<String>) -> &mut Self {
    if ids.is_empty() {
      return self
    }

    let mut event_ids: Vec<EventId> = vec![];
    for id in ids {
      event_ids.push(EventId(id));
    }

    self.ids = Some(event_ids);
    self
  }

  pub fn add_authors(&mut self, authors: Vec<String>) -> &mut Self {
    if authors.is_empty() {
      return self
    }

    self.authors = Some(authors);
    self
  }

  pub fn add_kinds(&mut self, kinds: Vec<u64>) -> &mut Self {
    if kinds.is_empty() {
      return self
    }

    let mut event_kinds: Vec<EventKind> = vec![];
    for kind in kinds {
      event_kinds.push(EventKind::from(kind));
    }

    self.kinds = Some(event_kinds);
    self
  }

  pub fn add_e_tags(&mut self, e_tags: Vec<String>) -> &mut Self {
    if e_tags.is_empty() {
      return self
    }

    self.e = Some(e_tags);
    self
  }

  pub fn add_p_tags(&mut self, p_tags: Vec<String>) -> &mut Self {
    if p_tags.is_empty() {
      return self
    }

    self.p = Some(p_tags);
    self
  }

  pub fn add_since(&mut self, since: u64) -> &mut Self {
    self.since = Some(since);
    self
  }

  pub fn add_until(&mut self, until: u64) -> &mut Self {
    self.until = Some(until);
    self
  }

  pub fn add_limit(&mut self, limit: u64) -> &mut Self {
    self.limit = Some(limit);
    self
  }

  pub fn as_str(&self) -> String {
    serde_json::to_string(self).unwrap()
  }

  pub fn from_string(data: String) -> Result<Self, serde_json::error::Error> {
    serde_json::from_str(&data)
  }

  pub fn from_string_array(data: String) -> Result<Vec<Self>, serde_json::error::Error> {
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
  fn test_filter_chaining_methods() {
    let ids = vec![String::from("id1"), String::from("id2")];
    let authors = vec![String::from("author1"), String::from("author2")];
    let kinds = vec![0, 1];
    let e_tags = vec![String::from("e_tag1"), String::from("e_tag2")];
    let p_tags = vec![String::from("p_tag1"), String::from("p_tag2")];
    let since = 10u64;
    let until = 11u64;
    let limit = 12u64;

    let mut filter_chained = Filter::new();
    filter_chained.add_ids(ids.clone()).add_authors(authors.clone()).add_kinds(kinds.clone()).add_e_tags(e_tags.clone()).add_p_tags(p_tags.clone()).add_since(since).add_until(until).add_limit(limit);

    assert_eq!(filter_chained.ids, Some(vec![EventId(ids[0].clone()), EventId(ids[1].clone())]));
    assert_eq!(filter_chained.authors, Some(authors));
    assert_eq!(filter_chained.kinds, Some(vec![EventKind::from(kinds[0]), EventKind::from(kinds[1])]));
    assert_eq!(filter_chained.e, Some(e_tags));
    assert_eq!(filter_chained.p, Some(p_tags));
    assert_eq!(filter_chained.since, Some(since));
    assert_eq!(filter_chained.until, Some(until));
    assert_eq!(filter_chained.limit, Some(limit));
  }

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
    // array
    let filter4 = json!(
      [{
        "#e": [
          "44b17a5acd66694cbdf5aea08968453658446368d978a15e61e599b8404d82c4",
          "7742783afbf6b283e81af63782ab0c05bbcbccba7f3abce0e0f23706dc27bd42",
          "9621051bcd8723f03da00aae61ee46956936726fcdfa6f34e29ae8f1e2b63cb5"
        ],
        "p": ["potato"],
        "kinds": [1, 6, 7, 9735]
      }])
      .to_string();

    let result = Filter::from_string(filter).unwrap();
    let result2 = Filter::from_string(filter2).unwrap();
    let result3 = Filter::from_string(filter3).unwrap();
    let result4 = Filter::from_string_array(filter4).unwrap();
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
    assert_eq!(result4, vec![expected]);
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
    assert_eq!(result["#e"], expected["e"]);
    assert_eq!(result["#p"], expected["#p"]);
    assert_eq!(result["authors"], expected["authors"]);
  }
}
