use crate::event::Event;

/// Used to send events requests by clients.
/// 
pub struct RelayToClientCommEvent {
  pub code: String, // "EVENT"
  pub subscription_id: String,
  pub event: Event
}

/// Used to indicate the End Of Stored Events (EOSE)
/// and the beginning of events newly received in
/// real-time.
/// 
pub struct RelayToClientCommEose {
  pub code: String, // "EOSE"
  pub subscription_id: String
}

/// Used to send human-readable error messages
/// or other things to clients.
/// 
pub struct RelayToClientCommNotice {
  pub code: String, // "NOTICE"
  pub message: String // NIP01 defines no rules for this message
}