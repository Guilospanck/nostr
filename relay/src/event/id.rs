use serde::{Deserialize, Serialize};
use bitcoin_hashes::{sha256, Hash};

use super::{PubKey, EventKind, Tag};

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct EventId(pub String);

impl EventId {
  ///
  /// This is the way used to serialize and get the SHA256. This will equal to `event.id`.
  /// 32-bytes lowercase hex-encoded sha256 of the the serialized event data
  ///
  /// <https://github.com/nostr-protocol/nips/blob/master/01.md>
  ///
  fn new(
    pubkey: PubKey,
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
