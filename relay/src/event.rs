use std::fmt;

use bitcoin_hashes::{sha256, Hash};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum EventError {
  #[error("kind invalid or not implemented")]
  KindNotFound,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct RecommendRelayURL(String);

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct PubKey(String);

/// Holds the types of `<marker>`
/// that an event tag (`"e"`) can have.
///
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, Eq)]
enum Marker {
  Root,
  Reply,
  Mention,
  #[default]
  Default,
}

impl fmt::Display for Marker {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Self::Root => write!(f, "root"),
      Self::Reply => write!(f, "reply"),
      Self::Mention => write!(f, "mention"),
      Self::Default => write!(f, ""),
    }
  }
}

impl<S> From<S> for Marker
where
  S: Into<String>,
{
  fn from(s: S) -> Self {
    let s: String = s.into();
    match s.as_str() {
      "root" => Self::Root,
      "reply" => Self::Reply,
      "mention" => Self::Mention,
      _ => Self::Default,
    }
  }
}

/// Holds the info of which are the
/// types (and order) of information that
/// an event tag `"e"` have.
///
enum EventTag {
  TagCode,
  EventId,
  RelayURL,
  Marker,
}

/// Holds the info of which are the
/// types (and order) of information that
/// a pubkey tag `"p"` have.
///
enum PubKeyTag {
  TagCode,
  ListOfPubKeys,
  RelayURL,
}

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
  ///   - `<relay-url>`: URL of a recommended relay associated with this reference. Ideally
  /// it would exist, but can be left with just `""`.
  ///   - `<marker>`: the type of event it is referencing. It is OPTIONAL. It can have three values:
  ///     - `root`: reply directly to the top-level event.
  ///     - `reply`: reply to some event, comment that is not the top-level one.
  ///     - `mention`: quoted or reposted event.
  ///
  Event,
}

impl fmt::Display for TagKind {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Self::PubKey => write!(f, "p"),
      Self::Event => write!(f, "e"),
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
    }
  }
}

///
/// This is the way used to serialize and get the SHA256. This will equal to `event.id`.
/// 32-bytes lowercase hex-encoded sha256 of the the serialized event data
///
/// <https://github.com/nostr-protocol/nips/blob/master/01.md>
///
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct EventId(String);

impl EventId {
  fn new(
    pubkey: String,
    created_at: u64,
    kind: EventKind,
    tags: Vec<Tag>,
    content: String,
  ) -> Self {
    let data = format!(
      "[{},\"{}\",{},{},{:?},\"{}\"]",
      0, pubkey, created_at, kind, tags, content
    );

    let hash = sha256::Hash::hash(&data.as_bytes());
    Self(hash.to_string())
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
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(untagged)]
pub enum Tag {
  Generic(TagKind, Vec<String>),
  Event(EventId, Option<RecommendRelayURL>, Option<Marker>),
  PubKey(PubKey, Option<RecommendRelayURL>),
}

impl<S> TryFrom<Vec<S>> for Tag
where
  S: Into<String>,
{
  type Error = EventError;

  fn try_from(tag: Vec<S>) -> Result<Self, Self::Error> {
    let tag: Vec<String> = tag.into_iter().map(|v| v.into()).collect();
    let tag_len: usize = tag.len();
    let tag_kind: TagKind = match tag.first() {
      Some(kind) => TagKind::from(kind),
      None => return Err(EventError::KindNotFound),
    };

    if tag_len == 2 {
      let content: String = tag[1].clone();
      match tag_kind {
        TagKind::PubKey => Ok(Self::PubKey(PubKey(content), None)),
        TagKind::Event => Ok(Self::Event(EventId(content), None, None)),
        _ => Ok(Self::Generic(tag_kind, vec![content.to_string()])),
      }
    } else if tag_len == 3 {
      match tag_kind {
        TagKind::PubKey => {
          let pubkey = PubKey(tag[1].clone());
          if tag[2].is_empty() {
            Ok(Self::PubKey(pubkey, Some(RecommendRelayURL::default())))
          } else {
            Ok(Self::PubKey(
              pubkey,
              Some(RecommendRelayURL(tag[2].clone())),
            ))
          }
        }
        TagKind::Event => Ok(Self::Event(
          EventId(tag[1].clone()),
          (!tag[2].is_empty()).then_some(RecommendRelayURL(tag[2].clone())),
          (!tag[3].is_empty()).then_some(Marker::from(&tag[3])),
        )),
        _ => Ok(Self::Generic(tag_kind, tag[1..].to_vec())),
      }
    } else if tag_len == 4 {
      match tag_kind {
        TagKind::Event => Ok(Self::Event(
          EventId(tag[1].clone()),
          (!tag[2].is_empty()).then_some(RecommendRelayURL(tag[2].clone())),
          (!tag[3].is_empty()).then_some(Marker::from(&tag[3])),
        )),
        _ => Ok(Self::Generic(tag_kind, tag[1..].to_vec())),
      }
    } else {
      Ok(Self::Generic(tag_kind, tag[1..].to_vec()))
    }
  }
}

/// Defines the type of the event.
/// Different types will change the meaning of different keys
/// of event object.
/// Default is `Text`.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EventKind {
  /// The content is set to a stringfied JSON object
  /// `{name: <username>, about: <string>, picture: <url, string>}`
  /// describing the user who created the event.
  /// A relay may delete past `Metadata` events once it gets a new one
  /// from the same pubkey.
  Metadata = 0,
  /// The content is set to the plaintext content of a note
  /// (anything the user wants to say). Markdown links (`[]()` stuff)
  /// are not plaintext.
  #[default]
  Text = 1,
  /// The content is set to the URL (e.g.: `wss://somerelay.com`) of a relay
  /// the event creator wants to recommend to its followers.
  RecommendRelay = 2,
}

impl fmt::Display for EventKind {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Self::Metadata => write!(f, "{}", 0),
      Self::Text => write!(f, "{}", 1),
      Self::RecommendRelay => write!(f, "{}", 2),
    }
  }
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
  pub pubkey: String,
  /// Unix timestamp in seconds
  pub created_at: u64,
  /// Kind of event
  pub kind: EventKind,
  /// An array of arrays with more info about the event,
  /// like, for example, if it is replying to someone.
  /// The kind of event will change the its tags and contents.
  pub tags: Vec<Tag>,
  /// Arbitrary string. Meaning depends on the kind of the event.
  pub content: String,
  /// 64-bytes hex signature of the id field
  pub sig: String,
}
