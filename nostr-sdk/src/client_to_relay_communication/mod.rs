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
/// 
use crate::{
  event::{
    tag::{Tag, TagKind},
    Event,
  },
  filter::Filter,
};

// Internal `client_to_relay_communication` modules
pub mod close;
pub mod event;
pub mod request;

/// [`ClientToRelayCommunication`] error
#[derive(thiserror::Error, Debug)]
pub enum Error {
  /// Error serializing or deserializing JSON data
  #[error(transparent)]
  Json(#[from] serde_json::Error),
  #[error("Invalid data")]
  InvalidData
}

impl serde::de::Error for Error {
  fn custom<T>(_msg:T) -> Self where T:std::fmt::Display {
      Self::InvalidData
  }
}

pub fn check_event_match_filter(event: Event, filter: Filter) -> bool {
  // Check IDs
  if let Some(ids) = filter.ids {
    let id_in_list = ids
      .iter()
      .any(|id| *id.0 == event.id || id.0.starts_with(&event.id));
    if !id_in_list {
      return false;
    }
  }

  // Check Authors
  if let Some(authors) = filter.authors {
    let author_in_list = authors
      .iter()
      .any(|author| *author == event.pubkey || author.starts_with(&event.pubkey));
    if !author_in_list {
      return false;
    }
  }

  // Check Kinds
  if let Some(kinds) = filter.kinds {
    let kind_in_list = kinds.iter().any(|kind| *kind == event.kind);
    if !kind_in_list {
      return false;
    }
  }

  // Check Since
  if let Some(since) = filter.since {
    let event_after_since = since <= event.created_at;
    if !event_after_since {
      return false;
    }
  }

  // Check Until
  if let Some(until) = filter.until {
    let event_before_until = until >= event.created_at;
    if !event_before_until {
      return false;
    }
  }

  // Check #e tag
  if let Some(event_ids) = filter.e {
    match event
      .tags
      .iter()
      .position(|event_tag| TagKind::from(event_tag.clone()) == TagKind::Event)
    {
      Some(index) => {
        if let Tag::Event(event_event_tag_id, _, _) = &event.tags[index] {
          if !event_ids
            .iter()
            .any(|event_id| *event_id == event_event_tag_id.0)
          {
            return false;
          }
        }
      }
      None => return false,
    }
  }

  // Check #p tag
  if let Some(pubkeys) = filter.p {
    match event
      .tags
      .iter()
      .position(|event_tag| TagKind::from(event_tag.clone()) == TagKind::PubKey)
    {
      Some(index) => {
        if let Tag::PubKey(event_pubkey_tag_pubkey, _) = &event.tags[index] {
          if !pubkeys
            .iter()
            .any(|pubkey| *pubkey == *event_pubkey_tag_pubkey)
          {
            return false;
          }
        }
      }
      None => return false,
    }
  }

  true
}

#[cfg(test)]
mod tests {
  use crate::{
    event::{id::EventId, kind::EventKind, Timestamp},
    filter::Filter,
  };

  use super::*;

  #[cfg(test)]
  use pretty_assertions::assert_eq;

  #[test]
  fn test_filter_match_ids() {
    let mock_filter_id = String::from("05b25af3-4250-4fbf-8ef5-97220858f9ab");
    let mock_filter_id2 = String::from("f6a54af2-1150-4fbf-8ef5-97220858f9ab");
    let filter = Filter {
      ids: Some(vec![EventId(mock_filter_id.clone())]),
      ..Default::default()
    };
    let event = Event {
      id: mock_filter_id,
      ..Default::default()
    };
    let event2 = Event {
      id: mock_filter_id2,
      ..Default::default()
    };

    assert_eq!(check_event_match_filter(event, filter.clone()), true);
    assert_eq!(check_event_match_filter(event2, filter), false);
  }

  #[test]
  fn test_filter_match_authors() {
    let mock_filter_author =
      String::from("02c7e1b1e9c175ab2d100baf1d5a66e73ecc044e9f8093d0c965741f26aa3abf76");
    let mock_filter_author2 =
      String::from("02c891b1e9c175ab2d100baf1d5a66e73ecc044e9f8093d0c965741f26aa3abf76");
    let filter = Filter {
      authors: Some(vec![mock_filter_author.clone()]),
      ..Default::default()
    };
    let event = Event {
      pubkey: mock_filter_author,
      ..Default::default()
    };
    let event2 = Event {
      pubkey: mock_filter_author2,
      ..Default::default()
    };

    assert_eq!(check_event_match_filter(event, filter.clone()), true);
    assert_eq!(check_event_match_filter(event2, filter), false);
  }

  #[test]
  fn test_filter_match_kinds() {
    let mock_filter_kind = 1;
    let mock_filter_kind2 = 2;
    let filter = Filter {
      kinds: Some(vec![EventKind::from(mock_filter_kind)]),
      ..Default::default()
    };
    let event = Event {
      kind: EventKind::from(mock_filter_kind),
      ..Default::default()
    };
    let event2 = Event {
      kind: EventKind::from(mock_filter_kind2),
      ..Default::default()
    };

    assert_eq!(check_event_match_filter(event, filter.clone()), true);
    assert_eq!(check_event_match_filter(event2, filter), false);
  }

  #[test]
  fn test_filter_match_since() {
    let mock_filter_since = 1683183423 as Timestamp;
    let filter = Filter {
      since: Some(mock_filter_since),
      ..Default::default()
    };
    let mock_created_at_after_since = 1693183423 as Timestamp;
    let event = Event {
      created_at: mock_created_at_after_since,
      ..Default::default()
    };
    let mock_created_at_before_since = 1673183423 as Timestamp;
    let event2 = Event {
      created_at: mock_created_at_before_since,
      ..Default::default()
    };

    assert_eq!(check_event_match_filter(event, filter.clone()), true);
    assert_eq!(check_event_match_filter(event2, filter), false);
  }

  #[test]
  fn test_filter_match_until() {
    let mock_filter_until = 1683183423 as Timestamp;
    let filter = Filter {
      until: Some(mock_filter_until),
      ..Default::default()
    };
    let mock_created_at_before_until = 1673183423 as Timestamp;
    let event = Event {
      created_at: mock_created_at_before_until,
      ..Default::default()
    };
    let mock_created_at_after_until = 1693183423 as Timestamp;
    let event2 = Event {
      created_at: mock_created_at_after_until,
      ..Default::default()
    };

    assert_eq!(check_event_match_filter(event, filter.clone()), true);
    assert_eq!(check_event_match_filter(event2, filter), false);
  }

  #[test]
  fn test_filter_e_tag() {
    let mock_filter_e_tag =
      String::from("ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb");
    let mock_filter_e_tag2 =
      String::from("da978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb");
    let filter = Filter {
      e: Some(vec![mock_filter_e_tag.clone()]),
      ..Default::default()
    };
    let event = Event {
      tags: vec![Tag::Event(EventId(mock_filter_e_tag), None, None)],
      ..Default::default()
    };
    let event2 = Event {
      tags: vec![Tag::Event(EventId(mock_filter_e_tag2), None, None)],
      ..Default::default()
    };

    assert_eq!(check_event_match_filter(event, filter.clone()), true);
    assert_eq!(check_event_match_filter(event2, filter), false);
  }

  #[test]
  fn test_filter_p_tag() {
    let mock_filter_p_tag =
      String::from("ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb");
    let mock_filter_p_tag2 =
      String::from("da978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb");
    let filter = Filter {
      p: Some(vec![mock_filter_p_tag.clone()]),
      ..Default::default()
    };
    let event = Event {
      tags: vec![Tag::PubKey(mock_filter_p_tag, None)],
      ..Default::default()
    };
    let event2 = Event {
      tags: vec![Tag::PubKey(mock_filter_p_tag2, None)],
      ..Default::default()
    };

    assert_eq!(check_event_match_filter(event, filter.clone()), true);
    assert_eq!(check_event_match_filter(event2, filter), false);
  }

  #[test]
  fn test_filter_should_match_all_requirements_to_be_true() {
    let mock_filter_id = String::from("05b25af3-4250-4fbf-8ef5-97220858f9ab");
    let mock_filter_author =
      String::from("02c7e1b1e9c175ab2d100baf1d5a66e73ecc044e9f8093d0c965741f26aa3abf76");
    let mock_filter_kind = 1;
    let mock_filter_since = 1663183423 as Timestamp;
    let mock_event_created_at_in_between = 1673183423 as Timestamp;
    let mock_filter_until = 1683183423 as Timestamp;
    let mock_filter_e_tag =
      String::from("ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb");
    let mock_filter_p_tag =
      String::from("02cd91b1e9c175ab2d100baf1d5a66e73ecc044e9f8093d0c965741f26aa3abf76");

    let filter = Filter {
      ids: Some(vec![EventId(mock_filter_id.clone())]),
      authors: Some(vec![mock_filter_author.clone()]),
      kinds: Some(vec![EventKind::from(mock_filter_kind)]),
      e: Some(vec![mock_filter_e_tag.clone()]),
      p: Some(vec![mock_filter_p_tag.clone()]),
      since: Some(mock_filter_since),
      until: Some(mock_filter_until),
      ..Default::default()
    };
    let event = Event {
      id: mock_filter_id,
      pubkey: mock_filter_author,
      kind: EventKind::from(mock_filter_kind),
      created_at: mock_event_created_at_in_between,
      tags: vec![
        Tag::PubKey(mock_filter_p_tag.clone(), None),
        Tag::Event(EventId(mock_filter_e_tag.clone()), None, None),
      ],
      ..Default::default()
    };

    assert_eq!(
      check_event_match_filter(event.clone(), filter.clone()),
      true
    );

    // different event id
    let mock_different_id = String::from("f6a54af2-1150-4fbf-8ef5-97220858f9ab");
    let event_different_id = Event {
      id: mock_different_id,
      ..event.clone()
    };

    assert_eq!(
      check_event_match_filter(event_different_id, filter.clone()),
      false
    );

    // different event author
    let mock_different_author =
      String::from("02e7e1b1e9c175ab2d100baf1d5a66e73ecc044e9f8093d0c965741f26aa3abf76");
    let event_different_author = Event {
      pubkey: mock_different_author,
      ..event.clone()
    };

    assert_eq!(
      check_event_match_filter(event_different_author, filter.clone()),
      false
    );

    // different event kind
    let mock_different_kind = 2;
    let event_different_kind = Event {
      kind: EventKind::from(mock_different_kind),
      ..event.clone()
    };

    assert_eq!(
      check_event_match_filter(event_different_kind, filter.clone()),
      false
    );

    // event created at outside of since-until range
    let mock_event_created_at_outside_range = 1773183423 as Timestamp;
    let event_different_created_at = Event {
      created_at: mock_event_created_at_outside_range,
      ..event.clone()
    };

    assert_eq!(
      check_event_match_filter(event_different_created_at, filter.clone()),
      false
    );

    // event different p tag
    let mock_event_different_p_tag =
      String::from("01cd91b1e9c175ab2d100baf1d5a66e73ecc044e9f8093d0c965741f26aa3abf76");
    let event_different_p_tag = Event {
      tags: vec![
        Tag::PubKey(mock_event_different_p_tag, None),
        Tag::Event(EventId(mock_filter_e_tag), None, None),
      ],
      ..event.clone()
    };

    assert_eq!(
      check_event_match_filter(event_different_p_tag, filter.clone()),
      false
    );

    // event different e tag
    let mock_event_different_e_tag =
      String::from("21cd91b1e9c175ab2d100baf1d5a66e73ecc044e9f8093d0c965741f26aa3abf76");
    let event_different_p_tag = Event {
      tags: vec![
        Tag::PubKey(mock_filter_p_tag, None),
        Tag::Event(EventId(mock_event_different_e_tag), None, None),
      ],
      ..event
    };

    assert_eq!(
      check_event_match_filter(event_different_p_tag, filter),
      false
    );
  }
}
