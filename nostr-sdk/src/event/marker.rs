use serde::{Deserialize, Serialize};
use std::fmt;

/// Holds the types of `<marker>`
/// that an event tag (`"e"`) can have.
///
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub enum Marker {
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
