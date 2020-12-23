use std::{alloc::Layout, path::Path};

use labrpc::{
    anyhow::Result,
    serde::{de::DeserializeOwned, Serialize},
    log::trace,
};
use rocksdb::DB;

pub struct Persistor {
    db: DB,
}

impl Persistor {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            db: DB::open_default(path).unwrap(),
        }
    }
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
        let mut p = new_persister();

        let key = String::from("key1");
        let value = String::from("value1");

        assert!(p.get::<_, String>(&key).unwrap().is_none());
        p.set(&key, &value).unwrap();
        assert!(p.get(key).unwrap() == Some(value));
    }

}