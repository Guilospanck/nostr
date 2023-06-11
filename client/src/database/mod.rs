use std::result;
pub mod keys_table;
pub mod subscriptions_table;

type Result<T> = result::Result<T, redb::Error>;

trait ClientDatabase<'a> {
  type K;
  type V;
  fn write_to_db(&self, k: Self::K, v: Self::V) -> Result<()>;
}

