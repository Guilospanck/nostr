use std::fs;

use redb::{Database, ReadableTable, TableDefinition, TableHandle};

use nostr_sdk::schnorr;

const TABLE: TableDefinition<&str, &str> = TableDefinition::new("keys");

#[derive(Debug, Default, Clone)]
pub struct Keys {
  pub private_key: String,
  pub public_key: String,
}

fn write_to_db(db: &Database, k: &str, v: &str) -> Result<(), redb::Error> {
  let write_txn = db.begin_write()?;
  {
    let mut table = write_txn.open_table(TABLE)?;
    table.insert(k, v)?;
  }
  write_txn.commit()?;
  Ok(())
}

pub fn get_client_keys() -> Result<Keys, redb::Error> {
  let mut keys = Keys::default();
  fs::create_dir_all("db/")?;
  let db = Database::create("db/client_db.redb")?;

  let read_txn = db.begin_read()?;
  let table_exists = read_txn
    .list_tables()?
    .any(|table| table.name() == TABLE.name());

  let table = {
    if table_exists {
      read_txn.open_table(TABLE)?
    } else {
      let write_txn = db.begin_write()?;
      write_txn.open_table(TABLE)?; // this basically just creates the table if doesn't exist
      write_txn.commit()?;
      read_txn.open_table(TABLE)?
    }
  };

  // try to get private key
  let private_key_kv = table.get("private_key").unwrap();
  let private_key = match private_key_kv {
    Some(private_key) => private_key.value().to_owned(),
    None => String::new(),
  };

  // try to get public keys
  let public_key_kv = table.get("public_key").unwrap();
  let public_key = match public_key_kv {
    Some(public_key) => public_key.value().to_owned(),
    None => String::new(),
  };

  // set keys
  keys.private_key = private_key;
  keys.public_key = public_key;

  // if keys are empty, generate new ones
  if String::is_empty(&keys.private_key) || String::is_empty(&keys.public_key) {
    let generated = schnorr::generate_keys();
    keys.private_key = generated.private_key.display_secret().to_string();
    keys.public_key = generated.public_key.to_string();

    write_to_db(&db, "private_key", &keys.private_key)?;
    write_to_db(&db, "public_key", &keys.public_key)?;
  }

  Ok(keys)
}
