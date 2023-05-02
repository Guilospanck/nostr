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
pub struct UncheckedRecommendRelayURL(String);

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
        TagKind::PubKey => {
          let pubkey = tag[1].clone();
          if tag[2].is_empty() {
            Ok(Self::PubKey(
              pubkey,
              Some(UncheckedRecommendRelayURL::default()),
            ))
          } else {
            Ok(Self::PubKey(
              pubkey,
              Some(UncheckedRecommendRelayURL(tag[2].clone())),
            ))
          }
        }
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
          event_tag.push(marker.to_string())
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
    let data: Vec<String> = self.clone().into();
    let mut seq = serializer.serialize_seq(Some(data.len()))?;
    for element in data.into_iter() {
      seq.serialize_element(&element)?;
    }
    seq.end()
  }
}

impl<'de> Deserialize<'de> for Tag {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    type Data = Vec<String>;
    let vec: Vec<String> = Data::deserialize(deserializer)?;
    Self::try_from(vec).map_err(DeserializerError::custom)
  }
}
