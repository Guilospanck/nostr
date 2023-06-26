use redb::{Database, ReadableTable, TableDefinition};
use std::{collections::HashMap, fs};

use crate::filter::Filter;

use super::{ClientDatabase, Result};

const TABLE_NAME: &str = "subscriptions";
const SUBSCRIPTIONS_TABLE: TableDefinition<&str, &str> = TableDefinition::new(TABLE_NAME);

#[derive(Debug)]
pub struct SubscriptionsTable {
  db: Database,
}

impl Default for SubscriptionsTable {
  fn default() -> Self {
    Self::new()
  }
}

impl<'a> ClientDatabase<'a> for SubscriptionsTable {
  type K = &'a str;
  type V = &'a str;

  fn write_to_db(&self, k: Self::K, v: Self::V) -> Result<()> {
    let write_txn = self.db.begin_write()?;
    {
      let mut table = write_txn.open_table(SUBSCRIPTIONS_TABLE)?;
      table.insert(k, v)?;
    }
    write_txn.commit()?;
    Ok(())
  }

  fn remove_from_db(&self, k: Self::K) -> Result<()> {
    let write_txn = self.db.begin_write()?;
    {
      let mut table = write_txn.open_table(SUBSCRIPTIONS_TABLE)?;
      table.remove(k)?;
    }
    write_txn.commit()?;
    Ok(())
  }
}

impl SubscriptionsTable {
  pub fn new() -> Self {
    fs::create_dir_all("db/").unwrap();
    let db = Database::create(format!("db/{TABLE_NAME}.redb")).unwrap();

    {
      let write_txn = db.begin_write().unwrap();
      write_txn.open_table(SUBSCRIPTIONS_TABLE).unwrap(); // this basically just creates the table if doesn't exist
      write_txn.commit().unwrap();
    }

    Self { db }
  }

  pub fn get_all_subscriptions(&self) -> Result<HashMap<String, Vec<Filter>>> {
    let mut subscriptions: HashMap<String, Vec<Filter>> = HashMap::new();
    let read_txn = self.db.begin_read()?;
    let table = read_txn.open_table(SUBSCRIPTIONS_TABLE)?;

    table.iter().unwrap().for_each(|subscription| {
      let subs = subscription.unwrap();
      let subs_id = subs.0.value();
      let subs_req_filters = subs.1.value();
      let filters_deserialized: Vec<Filter> =
        Filter::from_string_array(subs_req_filters.to_string()).unwrap();
      subscriptions.insert(subs_id.to_string(), filters_deserialized);
    });

    Ok(subscriptions)
  }

  pub fn add_new_subscription(&self, k: &str, v: &str) {
    self.write_to_db(k, v).unwrap();
  }

  pub fn remove_subscription(&self, k: &str) {
    self.remove_from_db(k).unwrap();
  }
}
