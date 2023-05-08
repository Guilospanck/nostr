use serde::{ser::SerializeSeq, Deserialize, Deserializer, Serialize, Serializer};


#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ClientToRelayCommClose {
  pub code: String, // "CLOSE"
  pub subscription_id: String,
}
