use std::fs;
use redb::{Database, ReadableTable, TableDefinition, WriteTransaction};

use nostr_sdk::event::Event;

pub struct EventsDB<'a> {
  table: TableDefinition<'a, u64, &'static str>,
  db: Database,
}

impl EventsDB<'_> {
  pub fn new() -> Result<Self, redb::Error> {
    fs::create_dir_all("db/")?;
    let db = Database::create("db/events.redb")?;
    const EVENTS_TABLE: TableDefinition<u64, &str> = TableDefinition::new("events");

    let write_txn = db.begin_write()?;
    write_txn.open_table(EVENTS_TABLE)?; // this basically just creates the table if doesn't exist
    write_txn.commit()?;

    Ok(Self {
      table: EVENTS_TABLE,
      db,
    })
  }

  fn begin_write(&self) -> Result<WriteTransaction, redb::Error> {
    self.db.begin_write()
  }

  fn commit_txn(&self, write_txn: WriteTransaction) -> Result<(), redb::Error> {
    write_txn.commit()
  }

  pub fn write_to_db(
    &mut self,
    k: u64,
    v: &str,
  ) -> Result<(), redb::Error> {
    let write_txn = self.begin_write()?;
    {
      let mut table = write_txn.open_table(self.table)?;
      table.insert(k, v)?;
    }
    self.commit_txn(write_txn)?;
    Ok(())
  }

  pub fn get_all_items(&self) -> Result<Vec<Event>, redb::Error> {
    let mut events: Vec<Event> = vec![];
    let read_txn = self.db.begin_read()?;
    let table = read_txn.open_table(self.table).unwrap();

    table.iter().unwrap().for_each(|event| {
      let evt = event.unwrap();
      let event_value = evt.1.value();
      let event_deserialized: Event = Event::from_json(event_value).unwrap();
      events.push(event_deserialized);
    });

    Ok(events)
  }
}
