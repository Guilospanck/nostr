use std::{fs, u8, vec};

use ::hex::decode;
use bitcoin_hashes::hex::ToHex;
use redb::{Database, ReadableTable, TableDefinition};

use nostr_sdk::schnorr;

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

impl KeysTable {
  pub fn new() -> Self {
    let keys = Keys::default();
    fs::create_dir_all("db/").unwrap();
    let db = Database::create(format!("db/{TABLE_NAME}.redb")).unwrap();

    {
      let write_txn = db.begin_write().unwrap();
      write_txn.open_table(KEYS_TABLE).unwrap(); // this basically just creates the table if doesn't exist
      write_txn.commit().unwrap();
    }

    Self { db, keys }
  }

  pub fn get_client_keys(&mut self) -> Result<Keys> {
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

    // set keys
    self.keys.private_key = private_key;
    self.keys.public_key = public_key;

    // if keys are empty, generate new ones
    if self.keys.private_key.is_empty() || self.keys.public_key.is_empty() {
      let generated = schnorr::generate_keys();
      self.keys.private_key = generated.private_key.secret_bytes().to_vec();
      let pubkey = &generated.public_key.to_hex()[2..];
      self.keys.public_key = decode(pubkey).unwrap();

      self.write_to_db("private_key", &self.keys.private_key)?;
      self.write_to_db("public_key", &self.keys.public_key)?;
    }

    Ok(self.keys.clone())
  }
}
