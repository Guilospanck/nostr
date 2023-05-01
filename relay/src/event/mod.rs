use serde::{Deserialize, Serialize};

// Event Modules
pub mod id;
pub mod kind;
pub mod tag;
mod marker;

use self::kind::EventKind;
use self::marker::Marker;
use self::id::EventId;
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
