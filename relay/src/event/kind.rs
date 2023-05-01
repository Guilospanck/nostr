use serde::de::{Deserialize, Deserializer, Error, Visitor};
use serde::ser::{Serialize, Serializer};
use std::fmt;
use std::num::ParseIntError;
use std::str::FromStr;

/// Defines the type of the event.
/// Different types will change the meaning of different keys
/// of event object.
/// `Text` is the default.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub enum EventKind {
  /// The content is set to a stringfied JSON object
  /// `{name: <username>, about: <string>, picture: <url, string>}`
  /// describing the user who created the event.
  /// A relay may delete past `Metadata` events once it gets a new one
  /// from the same pubkey.
  Metadata,
  /// The content is set to the plaintext content of a note
  /// (anything the user wants to say). Markdown links (`[]()` stuff)
  /// are not plaintext.
  #[default]
  Text,
  /// The content is set to the URL (e.g.: `wss://somerelay.com`) of a relay
  /// the event creator wants to recommend to its followers.
  RecommendRelay,
  /// A custom kind that we haven't implemented yet.
  Custom(u64),
}

// impl EventKind {
//   /// Get [`EventKind`] as `u32`
//   pub fn as_u32(&self) -> u32 {
//     self.as_u64() as u32
//   }

//   /// Get [`EventKind`] as `u64`
//   pub fn as_u64(&self) -> u64 {
//     (*self).into()
//   }
// }

impl FromStr for EventKind {
  type Err = ParseIntError;
  fn from_str(event_kind: &str) -> Result<Self, Self::Err> {
    let event_kind: u64 = event_kind.parse()?;
    Ok(Self::from(event_kind))
  }
}

impl From<u64> for EventKind {
  fn from(u: u64) -> Self {
    match u {
      0 => Self::Metadata,
      1 => Self::Text,
      2 => Self::RecommendRelay,
      x => Self::Custom(x),
    }
  }
}

impl From<EventKind> for u64 {
  fn from(e: EventKind) -> u64 {
    match e {
      EventKind::Metadata => 0,
      EventKind::Text => 1,
      EventKind::RecommendRelay => 2,
      EventKind::Custom(u) => u,
    }
  }
}

impl Serialize for EventKind {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    serializer.serialize_u64(From::from(*self))
  }
}

struct EventKindVisitor;

impl Visitor<'_> for EventKindVisitor {
  type Value = EventKind;

  fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "an unsigned number of maximum length of 64 bits")
  }

  fn visit_u64<E>(self, v: u64) -> Result<EventKind, E>
  where
    E: Error,
  {
    Ok(From::<u64>::from(v))
  }
}

impl<'de> Deserialize<'de> for EventKind {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    deserializer.deserialize_u64(EventKindVisitor)
  }
}

impl fmt::Display for EventKind {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Self::Metadata => write!(f, "0"),
      Self::Text => write!(f, "1"),
      Self::RecommendRelay => write!(f, "2"),
      Self::Custom(kind) => write!(f, "{kind}"),
      _ => write!(f, ""),
    }
  }
}
