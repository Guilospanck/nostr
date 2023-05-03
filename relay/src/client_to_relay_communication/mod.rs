use crate::{
  event::{
    tag::{Tag, TagKind},
    Event,
  },
  filter::Filter,
};

// Internal `client_to_relay_communication` modules
pub mod close;
pub mod event;
pub mod request;
pub mod types;

fn check_event_match_filter(event: Event, filter: Filter) -> bool {
  // Check IDs
  if let Some(ids) = filter.ids {
    let id_in_list = ids
      .iter()
      .any(|id| *id.0 == event.id || id.0.starts_with(&event.id));
    if !id_in_list {
      return false;
    }
  }

  // Check Authors
  if let Some(authors) = filter.authors {
    let author_in_list = authors
      .iter()
      .any(|author| *author == event.pubkey || author.starts_with(&event.pubkey));
    if !author_in_list {
      return false;
    }
  }

  // Check Kinds
  if let Some(kinds) = filter.kinds {
    let kind_in_list = kinds.iter().any(|kind| *kind == event.kind);
    if !kind_in_list {
      return false;
    }
  }

  // Check Since
  if let Some(since) = filter.since {
    let event_after_since = since <= event.created_at;
    if !event_after_since {
      return false;
    }
  }

  // Check Until
  if let Some(until) = filter.until {
    let event_before_until = until >= event.created_at;
    if !event_before_until {
      return false;
    }
  }

  // Check #e tag
  if let Some(event_ids) = filter.e {
    match event
      .tags
      .iter()
      .position(|event_tag| TagKind::from(event_tag.clone()) == TagKind::Event)
    {
      Some(index) => {
        if let Tag::Event(event_event_tag_id, _, _) = &event.tags[index] {
          if !event_ids
            .iter()
            .any(|event_id| *event_id == event_event_tag_id.0)
          {
            return false;
          }
        }
      }
      None => return false,
    }
  }

  // Check #p tag
  if let Some(pubkeys) = filter.p {
    match event
      .tags
      .iter()
      .position(|event_tag| TagKind::from(event_tag.clone()) == TagKind::PubKey)
    {
      Some(index) => {
        if let Tag::PubKey(event_pubkey_tag_pubkey, _) = &event.tags[index] {
          if !pubkeys
            .iter()
            .any(|pubkey| *pubkey == *event_pubkey_tag_pubkey)
          {
            return false;
          }
        }
      }
      None => return false,
    }
  }

  true
}
