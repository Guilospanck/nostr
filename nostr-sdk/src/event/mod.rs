use serde::{Deserialize, Serialize};

// Event Modules
pub mod id;
pub mod kind;
mod marker;
pub mod tag;

use self::id::EventId;
use self::kind::EventKind;
use self::marker::Marker;
use self::tag::Tag;

pub type PubKey = String;
pub type Timestamp = u64;

///
/// Event is the only object that exists in the Nostr protocol.
///
/// Example (id's and other hashes are not valid for the information presented):
///   ```json
///   {
///     "id": "ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb",
///     "pubkey": "02c7e1b1e9c175ab2d100baf1d5a66e73ecc044e9f8093d0c965741f26aa3abf76",
///     "created_at": 1673002822,
///     "kind": 1,
///     "tags": [
///       ["e", "688787d8ff144c502c7f5cffaafe2cc588d86079f9de88304c26b0cb99ce91c6", "wss://relay.damus.io", "root"],
///       ["p", "02c7e1b1e9c175ab2d100baf1d5a66e73ecc044e9f8093d0c965741f26aa3abf76", ""],
///     ],
///     "content": "Lorem ipsum dolor sit amet",
///     "sig": "e8551d85f530113366e8da481354c2756605e3f58149cedc1fb9385d35251712b954af8ef891cb0467d50ddc6685063d4190c97e9e131f903e6e4176dc13ce7c"
///   }
///   ```
///
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct Event {
  /// 32-bytes SHA256 of the serialized event data
  pub id: String,
  /// 32-bytes hex-encoded public key of the event creator  
  pub pubkey: PubKey,
  /// Unix timestamp in seconds
  pub created_at: Timestamp,
  /// Kind of event
  pub kind: EventKind,
  /// An array of arrays with more info about the event,
  /// like, for example, if it is replying to someone.
  /// The kind of event will change its tags and contents.
  pub tags: Vec<Tag>,
  /// Arbitrary string. Meaning depends on the kind of the event.
  pub content: String,
  /// 64-bytes hex signature of the id field
  pub sig: String,
}

impl Event {
  pub fn new_without_signature(
    pubkey: PubKey,
    created_at: Timestamp,
    kind: EventKind,
    tags: Vec<Tag>,
    content: String,
  ) -> Self {
    let id = EventId::new(
      pubkey.clone(),
      created_at,
      kind,
      tags.clone(),
      content.clone(),
    );
    Self {
      id: id.0,
      pubkey,
      created_at,
      kind,
      tags,
      content,
      ..Default::default()
    }
  }

  pub fn from_serialized(data: &str) -> Self {
    serde_json::from_str::<Self>(data).unwrap()
  }

  pub fn as_str(&self) -> String {
    serde_json::to_string(self).unwrap()
  }
}

#[cfg(test)]
mod tests {
  use super::{tag::UncheckedRecommendRelayURL, *};

  #[cfg(test)]
  use pretty_assertions::assert_eq;

  fn make_sut(
    tag_without_recommended_relay: bool,
    event_tag_without_marker: bool,
  ) -> (Event, String) {
    let mut expected_deserialized_event = Event {
      id: String::from("05b25af3-4250-4fbf-8ef5-97220858f9ab"),
      pubkey: PubKey::from("02c7e1b1e9c175ab2d100baf1d5a66e73ecc044e9f8093d0c965741f26aa3abf76"),
      created_at: 1673002822,
      kind: EventKind::Text,
      tags: vec![
        Tag::Event(EventId(String::from("688787d8ff144c502c7f5cffaafe2cc588d86079f9de88304c26b0cb99ce91c6")), Some(UncheckedRecommendRelayURL(String::from("wss://relay.damus.io"))), Some(Marker::Root)),
        Tag::PubKey(String::from("02c7e1b1e9c175ab2d100baf1d5a66e73ecc044e9f8093d0c965741f26aa3abf76"), Some(UncheckedRecommendRelayURL(String::from("wss://relay.damus.io"))))
      ],
      content: String::from("Lorem ipsum dolor sit amet"),
      sig: String::from("e8551d85f530113366e8da481354c2756605e3f58149cedc1fb9385d35251712b954af8ef891cb0467d50ddc6685063d4190c97e9e131f903e6e4176dc13ce7c")
    };

    let mut expected_serialized_event = r#"{"id":"05b25af3-4250-4fbf-8ef5-97220858f9ab","pubkey":"02c7e1b1e9c175ab2d100baf1d5a66e73ecc044e9f8093d0c965741f26aa3abf76","created_at":1673002822,"kind":1,"tags":[["e","688787d8ff144c502c7f5cffaafe2cc588d86079f9de88304c26b0cb99ce91c6","wss://relay.damus.io","root"],["p","02c7e1b1e9c175ab2d100baf1d5a66e73ecc044e9f8093d0c965741f26aa3abf76","wss://relay.damus.io"]],"content":"Lorem ipsum dolor sit amet","sig":"e8551d85f530113366e8da481354c2756605e3f58149cedc1fb9385d35251712b954af8ef891cb0467d50ddc6685063d4190c97e9e131f903e6e4176dc13ce7c"}"#.to_string();

    if tag_without_recommended_relay {
      expected_deserialized_event.tags = vec![
        Tag::Event(
          EventId(String::from(
            "688787d8ff144c502c7f5cffaafe2cc588d86079f9de88304c26b0cb99ce91c6",
          )),
          None,
          Some(Marker::Root),
        ),
        Tag::PubKey(
          String::from("02c7e1b1e9c175ab2d100baf1d5a66e73ecc044e9f8093d0c965741f26aa3abf76"),
          None,
        ),
      ];

      expected_serialized_event = r#"{"id":"05b25af3-4250-4fbf-8ef5-97220858f9ab","pubkey":"02c7e1b1e9c175ab2d100baf1d5a66e73ecc044e9f8093d0c965741f26aa3abf76","created_at":1673002822,"kind":1,"tags":[["e","688787d8ff144c502c7f5cffaafe2cc588d86079f9de88304c26b0cb99ce91c6","","root"],["p","02c7e1b1e9c175ab2d100baf1d5a66e73ecc044e9f8093d0c965741f26aa3abf76"]],"content":"Lorem ipsum dolor sit amet","sig":"e8551d85f530113366e8da481354c2756605e3f58149cedc1fb9385d35251712b954af8ef891cb0467d50ddc6685063d4190c97e9e131f903e6e4176dc13ce7c"}"#.to_string();
    }

    if event_tag_without_marker {
      expected_deserialized_event.tags = vec![
        Tag::Event(
          EventId(String::from(
            "688787d8ff144c502c7f5cffaafe2cc588d86079f9de88304c26b0cb99ce91c6",
          )),
          Some(UncheckedRecommendRelayURL(String::from(
            "wss://relay.damus.io",
          ))),
          None,
        ),
        Tag::PubKey(
          String::from("02c7e1b1e9c175ab2d100baf1d5a66e73ecc044e9f8093d0c965741f26aa3abf76"),
          Some(UncheckedRecommendRelayURL(String::from(
            "wss://relay.damus.io",
          ))),
        ),
      ];

      expected_serialized_event = r#"{"id":"05b25af3-4250-4fbf-8ef5-97220858f9ab","pubkey":"02c7e1b1e9c175ab2d100baf1d5a66e73ecc044e9f8093d0c965741f26aa3abf76","created_at":1673002822,"kind":1,"tags":[["e","688787d8ff144c502c7f5cffaafe2cc588d86079f9de88304c26b0cb99ce91c6","wss://relay.damus.io"],["p","02c7e1b1e9c175ab2d100baf1d5a66e73ecc044e9f8093d0c965741f26aa3abf76","wss://relay.damus.io"]],"content":"Lorem ipsum dolor sit amet","sig":"e8551d85f530113366e8da481354c2756605e3f58149cedc1fb9385d35251712b954af8ef891cb0467d50ddc6685063d4190c97e9e131f903e6e4176dc13ce7c"}"#.to_string();
    }

    (expected_deserialized_event, expected_serialized_event)
  }

  #[test]
  fn test_complete_event_serialize_and_deserialize_correctly() {
    let (expected_event, expected_serialized) = make_sut(false, false);
    assert_eq!(expected_event, Event::from_serialized(&expected_serialized));
    assert_eq!(expected_serialized, expected_event.as_str());
  }

  #[test]
  fn test_event_tags_without_relay_url_serialize_and_deserialize_correctly() {
    let (expected_event, expected_serialized) = make_sut(true, false);
    assert_eq!(expected_event, Event::from_serialized(&expected_serialized));
    assert_eq!(expected_serialized, expected_event.as_str());
  }

  #[test]
  fn test_event_tags_without_marker_and_deserialize_correctly() {
    let (expected_event, expected_serialized) = make_sut(false, true);
    assert_eq!(expected_event, Event::from_serialized(&expected_serialized));
    assert_eq!(expected_serialized, expected_event.as_str());
  }
}
