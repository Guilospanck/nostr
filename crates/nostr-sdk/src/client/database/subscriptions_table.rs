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
    Self::new(None)
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
  pub fn new(subscriptions_table_name: Option<String>) -> Self {
    fs::create_dir_all("db/").unwrap();
    let table_name = match subscriptions_table_name {
      Some(name) => name,
      None => TABLE_NAME.to_string(),
    };
    let db = Database::create(format!("db/{table_name}.redb")).unwrap();

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

#[cfg(test)]
mod tests {
  use std::vec;

  use super::*;

  #[cfg(test)]
  use pretty_assertions::assert_eq;

  struct Sut {
    subscription_id: String,
    filter_json: String,
    filters: Vec<Filter>,
    subscriptions_table: SubscriptionsTable,
    table_name: String,
  }

  impl Drop for Sut {
    fn drop(&mut self) {
      self.remove_temp_db();
    }
  }

  impl Sut {
    fn new(table_name: &str) -> Sut {
      let subscription_id = String::from("random-subs-id");
      let filter = Filter::new();
      let filters = vec![filter];
      let filters_json = serde_json::to_string(&filters).unwrap();

      let subscriptions_table = SubscriptionsTable::new(Some(table_name.to_string()));

      Sut {
        subscription_id,
        filter_json: filters_json,
        filters,
        subscriptions_table,
        table_name: table_name.to_string(),
      }
    }

    fn remove_temp_db(&self) {
      fs::remove_file(format!("db/{}.redb", self.table_name)).unwrap();
    }
  }

  #[test]
  fn write_to_db() {
    let sut = Sut::new("write_to_db_subscription_table");

    let result = sut
      .subscriptions_table
      .write_to_db(&sut.subscription_id, &sut.filter_json);
    assert!(result.is_ok());

    let all_subscriptions = sut.subscriptions_table.get_all_subscriptions();
    assert!(all_subscriptions.is_ok());

    assert_eq!(
      all_subscriptions.unwrap().get(&sut.subscription_id),
      Some(&sut.filters)
    );
  }

  #[test]
  fn remove_from_db() {
    let sut = Sut::new("remove_from_db_subscription_table");

    // add some data
    let result = sut
      .subscriptions_table
      .write_to_db(&sut.subscription_id, &sut.filter_json);
    assert!(result.is_ok());

    let result = sut.subscriptions_table.remove_from_db(&sut.subscription_id);
    assert!(result.is_ok());
    
    let all_subscriptions = sut.subscriptions_table.get_all_subscriptions();
    assert!(all_subscriptions.is_ok());
    assert!(all_subscriptions.unwrap().is_empty());
  }
}
