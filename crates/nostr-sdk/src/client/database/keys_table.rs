use std::{fs, u8, vec};

use ::hex::decode;
use bitcoin_hashes::hex::ToHex;
use redb::{Database, ReadableTable, TableDefinition};

use crate::schnorr;

use super::{ClientDatabase, Result};

const TABLE_NAME: &str = "keys";
const KEYS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new(TABLE_NAME);

#[derive(Debug, Default, Clone)]
pub struct Keys {
  pub private_key: Vec<u8>,
  pub public_key: Vec<u8>,
}

pub struct KeysTable {
  db: Database,
  keys: Keys,
}

impl<'a> ClientDatabase<'a> for KeysTable {
  type K = &'a str;
  type V = &'a [u8];

  fn write_to_db(&self, k: Self::K, v: Self::V) -> Result<()> {
    let write_txn = self.db.begin_write()?;
    {
      let mut table = write_txn.open_table(KEYS_TABLE)?;
      table.insert(k, v)?;
    }
    write_txn.commit()?;
    Ok(())
  }

  fn remove_from_db(&self, k: Self::K) -> Result<()> {
    let write_txn = self.db.begin_write()?;
    {
      let mut table = write_txn.open_table(KEYS_TABLE)?;
      table.remove(k)?;
    }
    write_txn.commit()?;
    Ok(())
  }
}

impl Default for KeysTable {
  fn default() -> Self {
    Self::new(None)
  }
}

impl KeysTable {
  pub fn new(keys_table_name: Option<String>) -> Self {
    let keys = Keys::default();
    fs::create_dir_all("db/").unwrap();
    let table_name = match keys_table_name {
      Some(name) => name,
      None => TABLE_NAME.to_string(),
    };
    let db = Database::create(format!("db/{table_name}.redb")).unwrap();

    {
      let write_txn = db.begin_write().unwrap();
      write_txn.open_table(KEYS_TABLE).unwrap(); // this basically just creates the table if doesn't exist
      write_txn.commit().unwrap();
    }

    Self { db, keys }
  }

  pub fn get_client_keys(&self) -> Result<Option<Keys>> {
    let read_txn = self.db.begin_read()?;
    let table = read_txn.open_table(KEYS_TABLE)?;

    // try to get private key
    let private_key_kv = table.get("private_key").unwrap();
    let private_key = match private_key_kv {
      Some(private_key) => private_key.value().to_owned(),
      None => vec![],
    };

    // try to get public keys
    let public_key_kv = table.get("public_key").unwrap();
    let public_key = match public_key_kv {
      Some(public_key) => public_key.value().to_owned(),
      None => vec![],
    };

    if private_key.is_empty() || public_key.is_empty() {
      return Ok(None)
    }

    Ok(Some(Keys {
      private_key,
      public_key,
    }))
  }

  pub fn get_or_create_client_keys(&mut self) -> Result<Keys> {
    let keys = self.get_client_keys()?;

    match keys {
      Some(keys) => {
        self.keys.private_key = keys.private_key;
        self.keys.public_key = keys.public_key;
      }
      None => {
        let generated = schnorr::generate_keys();
        self.keys.private_key = generated.private_key.secret_bytes().to_vec();
        let pubkey = &generated.public_key.to_hex()[2..];
        self.keys.public_key = decode(pubkey).unwrap();

        self.write_to_db("private_key", &self.keys.private_key)?;
        self.write_to_db("public_key", &self.keys.public_key)?;
      }
    }

    Ok(self.keys.clone())
  }
}

#[cfg(test)]
mod tests {
  use std::vec;

  use super::*;

  #[cfg(test)]
  use pretty_assertions::assert_eq;

  struct Sut {
    private_key: (String, Vec<u8>),
    public_key: (String, Vec<u8>),
    keys_table: KeysTable,
    table_name: String,
  }

  impl Drop for Sut {
    fn drop(&mut self) {
      self.remove_temp_db();
    }
  }

  impl Sut {
    fn new(table_name: &str) -> Sut {
      let public_key = (String::from("public_key"), vec![0u8, 1u8, 2u8, 3u8]);
      let private_key = (String::from("private_key"), vec![1u8, 2u8, 3u8, 4u8]);

      let keys_table = KeysTable::new(Some(table_name.to_string()));

      Sut {
        private_key,
        public_key,
        keys_table,
        table_name: table_name.to_string(),
      }
    }

    fn remove_temp_db(&self) {
      fs::remove_file(format!("db/{}.redb", self.table_name)).unwrap();
    }
  }

  #[test]
  fn write_to_db() {
    let sut = Sut::new("write_to_db");

    let result = sut
      .keys_table
      .write_to_db(&sut.private_key.0, &sut.private_key.1);
    assert!(result.is_ok());

    let result = sut
      .keys_table
      .write_to_db(&sut.public_key.0, &sut.public_key.1);
    assert!(result.is_ok());

    let keys = sut.keys_table.get_client_keys().unwrap();
    assert!(keys.is_some());

    assert_eq!(keys.clone().unwrap().private_key, sut.private_key.1);
    assert_eq!(keys.unwrap().public_key, sut.public_key.1);
  }

  #[test]
  fn remove_from_db() {
    let sut = Sut::new("remove_from_db");
    let mock_key = "potato";
    let mock_value = &[1u8, 2u8];

    // add some data
    let result = sut
      .keys_table
      .write_to_db(&sut.private_key.0, &sut.private_key.1);
    assert!(result.is_ok());

    let result = sut
      .keys_table
      .write_to_db(&sut.public_key.0, &sut.public_key.1);
    assert!(result.is_ok());

    let result = sut
      .keys_table
      .write_to_db(mock_key, mock_value);
    assert!(result.is_ok());

    // start removing data
    let result = sut.keys_table.remove_from_db(&sut.private_key.0);
    assert!(result.is_ok());

    let result = sut.keys_table.remove_from_db(&sut.public_key.0);
    assert!(result.is_ok());

    let keys = sut.keys_table.get_client_keys().unwrap();
    assert!(keys.is_none());
  }
}
