use redb::{Database, ReadableTable, TableDefinition, WriteTransaction};
use std::fs;

use crate::event::Event;

const TABLE_NAME: &str = "events";
const EVENTS_TABLE: TableDefinition<u64, &str> = TableDefinition::new("events");

pub struct EventsDB {
  db: Database,
}

impl EventsDB {
  pub fn new(events_table_name: Option<String>) -> Result<Self, redb::Error> {
    fs::create_dir_all("db/")?;
    let table_name = match events_table_name {
      Some(name) => name,
      None => TABLE_NAME.to_string(),
    };
    let db = Database::create(format!("db/{table_name}.redb"))?;

    let write_txn = db.begin_write()?;
    write_txn.open_table(EVENTS_TABLE)?; // this basically just creates the table if doesn't exist
    write_txn.commit()?;

    Ok(Self { db })
  }

  fn begin_write(&self) -> Result<WriteTransaction, redb::Error> {
    self.db.begin_write()
  }

  fn commit_txn(&self, write_txn: WriteTransaction) -> Result<(), redb::Error> {
    write_txn.commit()
  }

  pub fn write_to_db(&mut self, k: u64, v: &str) -> Result<(), redb::Error> {
    let write_txn = self.begin_write()?;
    {
      let mut table = write_txn.open_table(EVENTS_TABLE)?;
      table.insert(k, v)?;
    }
    self.commit_txn(write_txn)?;
    Ok(())
  }

  pub fn get_all_items(&self) -> Result<Vec<Event>, redb::Error> {
    let mut events: Vec<Event> = vec![];
    let read_txn = self.db.begin_read()?;
    let table = read_txn.open_table(EVENTS_TABLE).unwrap();

    table.iter().unwrap().for_each(|event| {
      let evt = event.unwrap();
      let event_value = evt.1.value();
      let event_deserialized: Event = Event::from_json(event_value).unwrap();
      events.push(event_deserialized);
    });

    Ok(events)
  }
}

#[cfg(test)]
mod tests {
  use std::vec;

  use super::*;

  #[cfg(test)]
  use pretty_assertions::assert_eq;
  use serde_json::json;

  struct Sut {
    events_db: EventsDB,
    table_name: String,
  }

  impl Drop for Sut {
    fn drop(&mut self) {
      self.remove_temp_db();
    }
  }

  impl Sut {
    fn new(table_name: &str) -> Self {
      let events_db = EventsDB::new(Some(table_name.to_string())).unwrap();

      Self {
        events_db,
        table_name: table_name.to_owned(),
      }
    }

    fn gen_event(&self) -> String {
      let event = Event::from_value(
        json!({"content":"potato","created_at":1684589418,"id":"00960bd35499f8c63a4f65e79d6b1a2b7f1b8c97e76652325567b78c496350ae","kind":1,"pubkey":"614a695bab54e8dc98946abdb8ec019599ece6dada0c23890977d0fa128081d6","sig":"bf073c935f71de50ec72bdb79f75b0bf32f9049305c3b22f97c06422c6f2edc86e0d7e07d7d7222678b238b1daee071be5f6fa653c611971395ec0d1c6407caf","tags":[]}),
      ).unwrap();
      event.as_json()
    }

    fn remove_temp_db(&self) {
      fs::remove_file(format!("db/{}.redb", self.table_name)).unwrap();
    }
  }

  #[test]
  fn write_to_db() {
    let mut sut = Sut::new("write_to_db");
    let mock_event = sut.gen_event();

    let result = sut.events_db.get_all_items().unwrap();
    assert_eq!(result.len(), 0);

    sut.events_db.write_to_db(0, &mock_event).unwrap();
    sut.events_db.write_to_db(1, &mock_event).unwrap();
    sut.events_db.write_to_db(2, &mock_event).unwrap();

    let result = sut.events_db.get_all_items().unwrap();
    assert_eq!(result.len(), 3);
  }

  #[test]
  fn get_all_items() {
    let sut = Sut::new("get_all_items");

    let result = sut.events_db.get_all_items().unwrap();

    assert_eq!(result.len(), 0);
  }
}
