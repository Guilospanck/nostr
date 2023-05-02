use serde::de::Error as DeserializerError;
use serde::{ser::SerializeSeq, Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt, vec};

use super::{EventId, Marker, PubKey};

/// [`Tag`] error
#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error("kind invalid or not implemented")]
  KindNotFound,
}

/// Holds the value of a Recommended Relay URL
/// that is send on an event.
///
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct UncheckedRecommendRelayURL(pub String);

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub enum TagKind {
  /// This pubkey tag is used to record who is involved in a reply thread.
  /// (Therefore it should only be used when the "e" tag is being used with
  /// `root` or `reply`).
  /// It has the following format:
  /// ```
  /// ["p", <pub-key> or <list-of-pub-keys-of-those-involved-in-the-reply-thread>, <relay-url>]
  /// ```
  ///
  PubKey,
  /// The event tag is used to, basically, reply to some other event.
  /// According to `NIP10`, which defines the `e` and `p` tags, it has
  /// the following format:
  /// ```
  /// ["e", <event-id>, <relay-url>, <marker>]
  /// ```
  ///
  /// where:
  ///   - `<event-id>`: id of the other event that this event is replying/mentioning to.
  ///   - `<relay-url>`: URL of a recommended relay associated with this reference.
  ///      It is OPTIONAL. Ideally it would exist, but can be left with just `""`.
  ///   - `<marker>`: the type of event it is referencing. It is OPTIONAL. It can have three values:
  ///     - `root`: reply directly to the top-level event.
  ///     - `reply`: reply to some event, comment that is not the top-level one.
  ///     - `mention`: quoted or reposted event.
  ///
  Event,
  /// Custom tag
  Custom(String),
}

impl fmt::Display for TagKind {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Self::PubKey => write!(f, "p"),
      Self::Event => write!(f, "e"),
      Self::Custom(tag) => write!(f, "{tag}"),
    }
  }
}

impl<S> From<S> for TagKind
where
  S: Into<String>,
{
  fn from(s: S) -> Self {
    let s: String = s.into();
    match s.as_str() {
      "p" => Self::PubKey,
      "e" => Self::Event,
      tag => Self::Custom(tag.to_string()),
    }
  }
}

impl From<Tag> for TagKind {
  fn from(data: Tag) -> Self {
    match data {
      Tag::Generic(kind, _) => kind,
      Tag::Event(_, _, _) => TagKind::Event,
      Tag::PubKey(_, _) => TagKind::PubKey,
    }
  }
}

/// A tag is dependent on the `EventKind`.
/// These are the ones used by EventKind=1 (Text):
///   - an EventTag (`"p"`, `"e"`)
///   - a string informing the content for that EventTag (pubkey for the "p" tag and event id for the "e" tag)
///   - an optional string of a recommended relay URL (can be set to "")
///   - an optional marker string for the "e" tag.
///
///   Example:
///
///   `["p", <32-bytes hex of the key>, <recommended relay URL>]`
///   ```json
///   ["p", "02c7e1b1e9c175ab2d100baf1d5a66e73ecc044e9f8093d0c965741f26aa3abf76", ""]
///   ```
///   
///   `["e", <32-bytes hex of the id of another event>, <recommended relay URL>, <marker>]`  
///   ```json
///   ["e", "688787d8ff144c502c7f5cffaafe2cc588d86079f9de88304c26b0cb99ce91c6", "wss://relay.damus.io", "root"]
///   ```
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tag {
  /// Generic because maybe the client is sending a tag that we
  /// don't have implemented yet.
  Generic(TagKind, Vec<String>),
  Event(EventId, Option<UncheckedRecommendRelayURL>, Option<Marker>),
  PubKey(PubKey, Option<UncheckedRecommendRelayURL>),
}

impl Tag {
  pub fn as_str(&self) -> String {
    serde_json::to_string(self).unwrap()
  }

  pub fn from_str(data: String) -> Self {
    serde_json::from_str(&data).unwrap()
  }

  pub fn as_vec(&self) -> Vec<String> {
    self.clone().into()
  }

  pub fn from_vec(data: Vec<String>) -> Self {
    Self::try_from(data).unwrap()
  }
}

impl<S> TryFrom<Vec<S>> for Tag
where
  S: Into<String>,
{
  type Error = Error;

  fn try_from(tag: Vec<S>) -> Result<Self, Self::Error> {
    let tag: Vec<String> = tag.into_iter().map(|v| v.into()).collect();
    let tag_len: usize = tag.len();
    let tag_kind: TagKind = match tag.first() {
      Some(kind) => TagKind::from(kind),
      None => return Err(Error::KindNotFound),
    };

    if tag_len == 1 {
      Ok(Self::Generic(tag_kind, vec![]))
    } else if tag_len == 2 {
      let content: String = tag[1].clone();
      match tag_kind {
        TagKind::PubKey => Ok(Self::PubKey(content, None)),
        TagKind::Event => Ok(Self::Event(EventId(content), None, None)),
        _ => Ok(Self::Generic(tag_kind, vec![content.to_string()])),
      }
    } else if tag_len == 3 {
      match tag_kind {
        TagKind::PubKey => Ok(Self::PubKey(
          tag[1].clone(),
          (!tag[2].is_empty()).then_some(UncheckedRecommendRelayURL(tag[2].clone())),
        )),
        TagKind::Event => Ok(Self::Event(
          EventId(tag[1].clone()),
          (!tag[2].is_empty()).then_some(UncheckedRecommendRelayURL(tag[2].clone())),
          None,
        )),
        _ => Ok(Self::Generic(tag_kind, tag[1..].to_vec())),
      }
    } else if tag_len == 4 {
      match tag_kind {
        TagKind::Event => Ok(Self::Event(
          EventId(tag[1].clone()),
          (!tag[2].is_empty()).then_some(UncheckedRecommendRelayURL(tag[2].clone())),
          (!tag[3].is_empty()).then_some(Marker::from(&tag[3])),
        )),
        _ => Ok(Self::Generic(tag_kind, tag[1..].to_vec())),
      }
    } else {
      Ok(Self::Generic(tag_kind, tag[1..].to_vec()))
    }
  }
}

impl From<Tag> for Vec<String> {
  fn from(data: Tag) -> Self {
    match data {
      Tag::Generic(kind, content) => vec![vec![kind.to_string()], content].concat(),
      Tag::Event(event_id, recommended_relay_url, marker) => {
        let mut event_tag = vec![TagKind::Event.to_string(), event_id.0];

        if let Some(url) = recommended_relay_url {
          event_tag.push(url.0);
        }

        if let Some(marker) = marker {
          if event_tag.len() == 2 {
            event_tag.push("".to_string());
          }
          event_tag.push(marker.to_string());
        }

        event_tag
      }
      Tag::PubKey(pubkey, recommended_relay_url) => {
        let mut pubkey_tag = vec![TagKind::PubKey.to_string(), pubkey];

        if let Some(url) = recommended_relay_url {
          pubkey_tag.push(url.0);
        }

        pubkey_tag
      }
    }
  }
}

impl Serialize for Tag {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    // using the `impl From<Tag> for Vec<String>`
    let data: Vec<String> = self.as_vec();
    // A Vec<_> is a sequence, therefore we must tell the
    // deserializer how long is the sequence (vector's length)
    let mut seq = serializer.serialize_seq(Some(data.len()))?;
    // Serialize each element of the Vector
    for element in data.into_iter() {
      seq.serialize_element(&element)?;
    }
    // Finalize the serialization and return the result
    seq.end()
  }
}

impl<'de> Deserialize<'de> for Tag {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    type Data = Vec<String>;
    // This is very intelligent. First it deserializes the enum
    // to something known, like a Vec<String> (serde library can handle this)
    // So in this case it is deserializing a string (serialized) into
    // a Vec<String>
    let vec: Vec<String> = Data::deserialize(deserializer)?;
    // Then it uses the `impl<S> TryFrom<Vec<S>> for Tag` to retrieve the `Tag` enum
    Self::try_from(vec).map_err(DeserializerError::custom)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[cfg(test)]
  use pretty_assertions::assert_eq;

  fn make_event_tag_sut(without_relay: bool, without_marker: bool) -> (Tag, String, Vec<String>) {
    let mut event = Tag::Event(
      EventId(String::from("event")),
      Some(UncheckedRecommendRelayURL(String::from("relay.com"))),
      Some(Marker::Root),
    );
    let mut serialized_event = "[\"e\",\"event\",\"relay.com\",\"root\"]".to_string();
    let mut expected_vector: Vec<String> = vec![
      String::from("e"),
      String::from("event"),
      String::from("relay.com"),
      String::from("root"),
    ];

    if without_relay && without_marker {
      event = Tag::Event(EventId(String::from("event")), None, None);
      serialized_event = "[\"e\",\"event\"]".to_string();
      expected_vector = vec![String::from("e"), String::from("event")];
    } else if without_relay {
      event = Tag::Event(EventId(String::from("event")), None, Some(Marker::Root));
      serialized_event = "[\"e\",\"event\",\"\",\"root\"]".to_string();
      expected_vector = vec![
        String::from("e"),
        String::from("event"),
        String::from(""),
        String::from("root"),
      ];
    } else if without_marker {
      event = Tag::Event(
        EventId(String::from("event")),
        Some(UncheckedRecommendRelayURL(String::from("relay.com"))),
        None,
      );
      serialized_event = "[\"e\",\"event\",\"relay.com\"]".to_string();
      expected_vector = vec![
        String::from("e"),
        String::from("event"),
        String::from("relay.com"),
      ];
    }

    (event, serialized_event, expected_vector)
  }

  fn make_pubkey_tag_sut(without_relay: bool) -> (Tag, String, Vec<String>) {
    let mut pubkey = Tag::PubKey(
      String::from("pubkey"),
      Some(UncheckedRecommendRelayURL(String::from("relay.com"))),
    );
    let mut expected_pubkey: String = "[\"p\",\"pubkey\",\"relay.com\"]".to_string();
    let mut expected_vector: Vec<String> = vec![
      String::from("p"),
      String::from("pubkey"),
      String::from("relay.com"),
    ];

    if without_relay {
      pubkey = Tag::PubKey(String::from("pubkey"), None);
      expected_pubkey = "[\"p\",\"pubkey\"]".to_string();
      expected_vector = vec![String::from("p"), String::from("pubkey")];
    }

    (pubkey, expected_pubkey, expected_vector)
  }

  #[test]
  fn test_tag_serializes_and_deserializes_correctly() {
    // Generic - serialization
    let generic = Tag::Generic(
      TagKind::Custom(String::from("custom_tag")),
      vec![String::from("potato"), String::from("cake")],
    );
    let expected_generic: String = "[\"custom_tag\",\"potato\",\"cake\"]".to_string();
    assert_eq!(generic.as_str(), expected_generic);

    // Generic - deserialization
    assert_eq!(Tag::from_str(expected_generic), generic);

    // Pubkey - serialization
    let (pubkey_without_recommended_relay, expected_pubkey_without_recommended_relay, _) =
      make_pubkey_tag_sut(true);
    let (pubkey_complete, expected_pubkey_complete, _) = make_pubkey_tag_sut(false);
    assert_eq!(
      pubkey_without_recommended_relay.as_str(),
      expected_pubkey_without_recommended_relay
    );
    assert_eq!(pubkey_complete.as_str(), expected_pubkey_complete);

    // Pubkey - deserialization
    assert_eq!(
      Tag::from_str(expected_pubkey_without_recommended_relay),
      pubkey_without_recommended_relay
    );
    assert_eq!(Tag::from_str(expected_pubkey_complete), pubkey_complete);

    // Event - serialization
    let (
      event_without_recommended_relay_and_marker,
      expected_event_without_recommended_relay_and_marker,
      _,
    ) = make_event_tag_sut(true, true);
    let (event_complete_without_marker, expected_event_complete_without_marker, _) =
      make_event_tag_sut(false, true);
    let (
      event_complete_without_recommended_relay,
      expected_event_complete_without_recommended_relay,
      _,
    ) = make_event_tag_sut(true, false);
    let (event_complete, expected_event_complete, _) = make_event_tag_sut(false, false);
    assert_eq!(
      event_without_recommended_relay_and_marker.as_str(),
      expected_event_without_recommended_relay_and_marker
    );
    assert_eq!(
      event_complete_without_marker.as_str(),
      expected_event_complete_without_marker
    );
    assert_eq!(
      event_complete_without_recommended_relay.as_str(),
      expected_event_complete_without_recommended_relay
    );
    assert_eq!(event_complete.as_str(), expected_event_complete);

    // Event - deserialization
    assert_eq!(
      Tag::from_str(expected_event_without_recommended_relay_and_marker),
      event_without_recommended_relay_and_marker
    );
    assert_eq!(
      Tag::from_str(expected_event_complete_without_marker),
      event_complete_without_marker
    );
    assert_eq!(
      Tag::from_str(expected_event_complete_without_recommended_relay),
      event_complete_without_recommended_relay
    );
    assert_eq!(Tag::from_str(expected_event_complete), event_complete);
  }

  #[test]
  fn test_tag_as_a_vector_and_it_as_a_tag() {
    // Generic - as_vec
    let generic = Tag::Generic(
      TagKind::Custom(String::from("custom_tag")),
      vec![String::from("potato"), String::from("cake")],
    );
    let expected_generic_vector: Vec<String> = vec![
      String::from("custom_tag"),
      String::from("potato"),
      String::from("cake"),
    ];
    assert_eq!(generic.as_vec(), expected_generic_vector);

    // Generic - as_vec
    assert_eq!(generic, Tag::from_vec(expected_generic_vector));

    // Pubkey - as_vec
    let (pubkey_tag_complete, _, expected_pubkey_tag_complete_vector) = make_pubkey_tag_sut(false);
    let (pubkey_tag_without_relay, _, expected_pubkey_tag_without_relay_vector) =
      make_pubkey_tag_sut(true);
    assert_eq!(
      pubkey_tag_complete.as_vec(),
      expected_pubkey_tag_complete_vector
    );
    assert_eq!(
      pubkey_tag_without_relay.as_vec(),
      expected_pubkey_tag_without_relay_vector
    );

    // Pubkey - as_vec
    assert_eq!(
      pubkey_tag_complete,
      Tag::from_vec(expected_pubkey_tag_complete_vector)
    );
    assert_eq!(
      pubkey_tag_without_relay,
      Tag::from_vec(expected_pubkey_tag_without_relay_vector)
    );

    // Event - as_vec
    let (
      event_without_recommended_relay_and_marker,
      _,
      expected_event_without_recommended_relay_and_marker_vector,
    ) = make_event_tag_sut(true, true);
    let (event_complete_without_marker, _, expected_event_complete_without_marker_vector) =
      make_event_tag_sut(false, true);
    let (
      event_complete_without_recommended_relay,
      _,
      expected_event_complete_without_recommended_relay_vector,
    ) = make_event_tag_sut(true, false);
    let (event_complete, _, expected_event_complete_vector) = make_event_tag_sut(false, false);
    assert_eq!(
      event_without_recommended_relay_and_marker.as_vec(),
      expected_event_without_recommended_relay_and_marker_vector
    );
    assert_eq!(
      event_complete_without_marker.as_vec(),
      expected_event_complete_without_marker_vector
    );
    assert_eq!(
      event_complete_without_recommended_relay.as_vec(),
      expected_event_complete_without_recommended_relay_vector
    );
    assert_eq!(event_complete.as_vec(), expected_event_complete_vector);

    // Event - as_vec
    assert_eq!(
      event_without_recommended_relay_and_marker,
      Tag::from_vec(expected_event_without_recommended_relay_and_marker_vector)
    );
    assert_eq!(
      event_complete_without_marker,
      Tag::from_vec(expected_event_complete_without_marker_vector)
    );
    assert_eq!(
      event_complete_without_recommended_relay,
      Tag::from_vec(expected_event_complete_without_recommended_relay_vector)
    );
    assert_eq!(
      event_complete,
      Tag::from_vec(expected_event_complete_vector)
    );
  }
}
