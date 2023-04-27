use bitcoin_hashes::{sha256, Hash};
use serde::{Deserialize, Serialize};

pub enum EventTags {
  PubKey,
  Event,
}

impl EventTags {
  fn as_str(&self) -> &'static str {
    match self {
      EventTags::PubKey => "p", // points to a pubkey of someone that is referred to in the event
      EventTags::Event => "e", // points to the id of an event this event is quoting, replying to or referring to somehow.
    }
  }
}

/// A tag is made of 3 parts:
///   - an EventTag (p, e ...)
///   - a string informing the content for that EventTag (pubkey for the "p" tag and event id for the "e" tag)
///   - an optional string of a recommended relay URL (can be set to "")
///
///   ```["p", <32-bytes hex of the key>, <recommended relay URL>]```
///   ```["e", <32-bytes hex of the id of another event>, <recommended relay URL>]```
///
///   Example:
///   ```json
///   ["e", "688787d8ff144c502c7f5cffaafe2cc588d86079f9de88304c26b0cb99ce91c6", "wss://relay.damus.io"]
///   ["p", "02c7e1b1e9c175ab2d100baf1d5a66e73ecc044e9f8093d0c965741f26aa3abf76", ""]
///   ```
///
pub type Tag = [String; 3];

pub enum EventKinds {
  Metadata = 0,
  Text = 1,
  RecommendRelay = 2,
  Contacts = 3,
  EncryptedDirectMessages = 4,
  EventDeletion = 5,
  Repost = 6,
  Reaction = 7,
  ChannelCreation = 40,
  ChannelMetadata = 41,
  ChannelMessage = 42,
  ChannelHideMessage = 43,
  ChannelMuteUser = 44,
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
///       ["e", "688787d8ff144c502c7f5cffaafe2cc588d86079f9de88304c26b0cb99ce91c6", "wss://relay.damus.io"],
///       ["p", "02c7e1b1e9c175ab2d100baf1d5a66e73ecc044e9f8093d0c965741f26aa3abf76", ""],
///     ],
///     "content": "Lorem ipsum dolor sit amet",
///     "sig": "e8551d85f530113366e8da481354c2756605e3f58149cedc1fb9385d35251712b954af8ef891cb0467d50ddc6685063d4190c97e9e131f903e6e4176dc13ce7c"
///   }
///   ```
///
#[derive(Debug, Deserialize, Serialize, Default, Clone, PartialEq, Eq)]
pub struct Event {
  pub id: String,      // 32-bytes SHA256 of the serialized event data
  pub pubkey: String,  // 32-bytes hex-encoded public key of the event creator
  pub created_at: u64, // unix timestamp in seconds
  pub kind: u64,       // kind of event
  pub tags: Vec<Tag>,
  pub content: String, // arbitrary string
  pub sig: String,     // 64-bytes hex signature of the id field
}

impl Event {
  ///
  /// This is the way used to serialize and get the SHA256. This will equal to `event.id`.
  ///
  fn get_id(&self) -> String {
    let data = format!(
      "[{},\"{}\",{},{},{:?},\"{}\"]",
      0, self.pubkey, self.created_at, self.kind, self.tags, self.content
    );

    let hash = sha256::Hash::hash(&data.as_bytes());
    hash.to_string()
  }
}