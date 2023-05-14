use bitcoin_hashes::{sha256, Hash};
use serde::{Deserialize, Serialize};

use super::{kind::EventKind, tag::Tag, PubKey, Timestamp};

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct EventId(pub String);

impl EventId {
  ///
  /// This is the way used to serialize and get the SHA256. This will equal to `event.id`.
  /// 32-bytes lowercase hex-encoded sha256 of the the serialized event data
  ///
  /// <https://github.com/nostr-protocol/nips/blob/master/01.md>
  ///
  pub(crate) fn new(
    pubkey: PubKey,
    created_at: Timestamp,
    kind: EventKind,
    tags: Vec<Tag>,
    content: String,
  ) -> Self {
    let data = format!(
      "[{},\"{}\",{},{},{:?},\"{}\"]",
      0, pubkey, created_at, kind, tags, content
    );

    let hash = sha256::Hash::hash(data.as_bytes());
    Self(hash.to_string())
  }
}

#[cfg(test)]
mod tests {

  use crate::event::{marker::Marker, tag::UncheckedRecommendRelayURL};

  use super::*;

  #[cfg(test)]
  use pretty_assertions::assert_eq;

  #[test]
  fn creates_id() {
    let mock_pub_key: PubKey = String::from("mockpubkey");
    let mock_created_at: Timestamp = 161500343030;
    let mock_kind: EventKind = EventKind::Text;
    let mock_tags: Vec<Tag> = vec![Tag::Event(
      EventId(String::from("event_im_replying_to")),
      Some(UncheckedRecommendRelayURL(String::from(
        "wss://recommended.relay.com",
      ))),
      Some(Marker::Reply),
    )];
    let mock_content: String = String::from("mockcontent");

    let event_id = EventId::new(
      mock_pub_key.clone(),
      mock_created_at,
      mock_kind,
      mock_tags.clone(),
      mock_content.clone(),
    );
    let expected = format!(
      "[{},\"{}\",{},{},{:?},\"{}\"]",
      0, mock_pub_key, mock_created_at, mock_kind, mock_tags, mock_content
    );
    let not_expected = EventId(sha256::Hash::hash(format!(
      "[{},\"{}\",{},{},{:?},\"{}\"]",
      1, mock_pub_key, mock_created_at, mock_kind, mock_tags, mock_content
    ).as_bytes()).to_string());
    let hash = sha256::Hash::hash(expected.as_bytes());
    let expected = EventId(hash.to_string());

    assert_eq!(expected, event_id);
    assert_ne!(not_expected, event_id);
  }
}
