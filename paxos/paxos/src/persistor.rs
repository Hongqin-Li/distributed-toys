use std::path::Path;

use labrpc::serde::{de::DeserializeOwned, Serialize};
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
    pub fn get<T>(&self, key: &str) -> Option<T>
    where
        T: DeserializeOwned + Clone,
    {
        self.db
            .get(key)
            .unwrap()
            .map(|b| bincode::deserialize(&b).unwrap())
    }
    pub fn set<T: Serialize>(&self, key: &str, value: &T) {
        self.db
            .put(key, bincode::serialize(value).unwrap())
            .unwrap();
    }
}
