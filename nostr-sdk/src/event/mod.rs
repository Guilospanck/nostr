use std::str::FromStr;

use secp256k1::{schnorr, Secp256k1};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

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

/// [`Event`] error
#[derive(thiserror::Error, Debug)]
pub enum Error {
  /// Error serializing or deserializing JSON data
  #[error(transparent)]
  Json(#[from] serde_json::Error),
  #[error("Invalid data")]
  InvalidData,
}

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

  pub fn sign_event(&mut self, seckey: Vec<u8>) {
    let secp = Secp256k1::new();
    let msg = self.id.clone();
    let signed = crate::schnorr::sign_schnorr(&secp, msg, seckey).unwrap();
    self.sig = signed.to_string();
  }

  pub fn check_event_id(&self) -> bool {
    EventId::new(
      self.pubkey.clone(),
      self.created_at,
      self.kind,
      self.tags.clone(),
      self.content.clone(),
    )
    .0 == self.id
  }

  pub fn check_event_signature(&self) -> bool {
    let secp = Secp256k1::new();
    let sig = match schnorr::Signature::from_str(&self.sig) {
      Ok(signature) => signature,
      Err(_) => return false,
    };
    let msg = self.id.clone();

    crate::schnorr::verify_schnorr(&secp, msg, sig, self.pubkey.clone())
      .unwrap_or(false)
  }

  /// Deserializes from [`Value`]
  pub fn from_value(msg: Value) -> Result<Self, Error> {
    serde_json::from_value(msg).map_err(Error::Json)
  }

  /// Serialize as [`Value`]
  pub fn as_value(&self) -> Value {
    json!(self)
  }

  /// Deserialize [`Event`] from JSON string
  pub fn from_json<S>(msg: S) -> Result<Self, Error>
  where
    S: Into<String>,
  {
    let msg: &str = &msg.into();

    if msg.is_empty() {
      return Err(Error::InvalidData);
    }

    let value: Value = serde_json::from_str(msg)?;
    Self::from_value(value)
  }

  /// Get [`Event`] in JSON string
  pub fn as_json(&self) -> String {
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
    assert_eq!(
      expected_event,
      Event::from_json(&expected_serialized).unwrap()
    );
    assert_eq!(expected_serialized, expected_event.as_json());
  }

  #[test]
  fn test_event_tags_without_relay_url_serialize_and_deserialize_correctly() {
    let (expected_event, expected_serialized) = make_sut(true, false);
    assert_eq!(
      expected_event,
      Event::from_json(&expected_serialized).unwrap()
    );
    assert_eq!(expected_serialized, expected_event.as_json());
  }

  #[test]
  fn test_event_tags_without_marker_and_deserialize_correctly() {
    let (expected_event, expected_serialized) = make_sut(false, true);
    assert_eq!(
      expected_event,
      Event::from_json(&expected_serialized).unwrap()
    );
    assert_eq!(expected_serialized, expected_event.as_json());
  }

  #[test]
  fn check_event_id() {
    let (expected_event, _) = make_sut(false, true);
    assert_eq!(expected_event.check_event_id(), false);

    let event_with_correct_signature = Event::from_value(
      json!({"content":"potato","created_at":1684589418,"id":"00960bd35499f8c63a4f65e79d6b1a2b7f1b8c97e76652325567b78c496350ae","kind":1,"pubkey":"614a695bab54e8dc98946abdb8ec019599ece6dada0c23890977d0fa128081d6","sig":"bf073c935f71de50ec72bdb79f75b0bf32f9049305c3b22f97c06422c6f2edc86e0d7e07d7d7222678b238b1daee071be5f6fa653c611971395ec0d1c6407caf","tags":[]}),
    ).unwrap();
    assert_eq!(event_with_correct_signature.check_event_id(), true);
  }

  #[test]
  fn check_event_signature() {
    let (expected_event, _) = make_sut(false, true);
    assert_eq!(expected_event.check_event_signature(), false);

    let event_with_correct_signature = Event::from_value(
      json!({"content":"potato","created_at":1684589418,"id":"00960bd35499f8c63a4f65e79d6b1a2b7f1b8c97e76652325567b78c496350ae","kind":1,"pubkey":"614a695bab54e8dc98946abdb8ec019599ece6dada0c23890977d0fa128081d6","sig":"bf073c935f71de50ec72bdb79f75b0bf32f9049305c3b22f97c06422c6f2edc86e0d7e07d7d7222678b238b1daee071be5f6fa653c611971395ec0d1c6407caf","tags":[]}),
    ).unwrap();
    assert_eq!(event_with_correct_signature.check_event_signature(), true);
  }

  #[test]
  fn sign_event() {
    let event_sut = make_sut(false, false);
    let keys = crate::schnorr::generate_keys();
    // In order to use Schnorr signatures, we have to drop the first byte of pubkey
    let pubkey = &keys.public_key.to_string()[2..];
    let mut event = Event::new_without_signature(
      pubkey.to_string(),
      event_sut.0.created_at,
      event_sut.0.kind,
      event_sut.0.tags,
      event_sut.0.content
    );

    event.sign_event(keys.private_key.secret_bytes().to_vec());

    assert_eq!(event.check_event_signature(), true);
  }
}
