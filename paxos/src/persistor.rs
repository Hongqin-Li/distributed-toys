use std::path::Path;

use labrpc::{
    anyhow::Result,
    serde::{de::DeserializeOwned, Serialize},
};
use rocksdb::DB;

/// A wrapper of RocksDB which provides set, get, remove operations
/// for type that derive [Serialize](serde::Serialize) and [Deserialize](serde::Deserialize)
pub struct Persistor {
    db: DB,
}

impl Persistor {
    /// Create a new persistor with given path to a file.
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            db: DB::open_default(path).unwrap(),
        }
    }
    /// Get a value by given key.
    pub fn get<K, T>(&self, key: K) -> Result<Option<T>>
    where
        K: AsRef<[u8]>,
        T: DeserializeOwned + Clone,
    {
        let opt = self.db.get(key)?;
        if let Some(v) = opt {
            Ok(Some(bincode::deserialize(&v)?))
        } else {
            Ok(None)
        }
    }
    /// Set value associated to given key.
    pub fn set<K: AsRef<[u8]>, T: Serialize>(&self, key: K, value: &T) -> Result<()> {
        Ok(self.db.put(key, bincode::serialize(value)?)?)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn new_persister() -> Persistor {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test");
        Persistor::new(path)
    }
    #[test]
    fn test_all() {
        let p = new_persister();

        let key = String::from("key1");
        let value = String::from("value1");

        assert!(p.get::<_, String>(&key).unwrap().is_none());
        p.set(&key, &value).unwrap();
        assert!(p.get(key).unwrap() == Some(value));
    }
}
