use super::KvService;

use labrpc::{
    anyhow::Result,
    serde::{Deserialize, Serialize},
    serde_json, Server,
};
use paxos::{Acceptor, AcceptorClient, AcceptorServer, Proposer, ProposerService};
use rocksdb::{WriteBatch, DB};
use std::path::{Path, PathBuf};

pub struct Paxoskv {
    db: DB,
    // acceptor: AcceptorServer<Acceptor>,
    proposer: Proposer,
    id: u32,

    // Persisted data
    term: u64,
}

#[derive(Clone, Serialize, Deserialize)]
struct LogEntry {
    pid: u32,
    op: Operation,
}

#[derive(Clone, Serialize, Deserialize)]
enum Operation {
    Get { key: String },
    Set { key: String, value: String },
    Remove { key: String },
}

pub struct ClusterInfo {
    pub acc_clients: Vec<AcceptorClient>,
}

impl Paxoskv {
    fn new(dir: impl Into<PathBuf>, id: u32, cluster: ClusterInfo) -> Self {
        let dir: PathBuf = dir.into();
        let db_path = dir.join("paxoskv");

        let proposer = Proposer::new(id, cluster.acc_clients);
        let db = DB::open_default(db_path).unwrap();
        let term = db
            .get("term")
            .unwrap()
            .map_or(0, |v| bincode::deserialize(&v).unwrap());

        Self {
            id,
            db,
            term,
            proposer,
        }
    }
}

#[labrpc::async_trait]
impl KvService for Paxoskv {
    async fn get_local(&mut self, key: String) -> Result<Option<String>> {
        if let Some(v) = self.db.get(&key)? {
            Ok(Some(String::from_utf8(v)?))
        } else {
            Ok(None)
        }
    }
    async fn get(&mut self, key: String) -> Result<Option<String>> {
        let ent = LogEntry {
            pid: self.id,
            op: Operation::Get { key: key.clone() },
        };
        loop {
            self.term += 1;

            let prev = self
                .proposer
                .choose(self.term, serde_json::to_string(&ent)?)
                .await?;
            let prev: LogEntry = serde_json::from_str(&prev)?;
            if prev.pid == self.id {
                let opt = self.db.get(&key)?;
                break {
                    if let Some(v) = opt {
                        Ok(Some(String::from_utf8(v)?))
                    } else {
                        Ok(None)
                    }
                };
            }
        }
    }
    async fn set(&mut self, key: String, value: String) -> Result<Option<String>> {
        let ent = LogEntry {
            pid: self.id,
            op: Operation::Set {
                key: key.clone(),
                value: value.clone(),
            },
        };
        loop {
            self.term += 1;

            let prev = self
                .proposer
                .choose(self.term, serde_json::to_string(&ent)?)
                .await?;
            let prev: LogEntry = serde_json::from_str(&prev)?;
            if prev.pid == self.id {
                if let Operation::Get { key } = prev.op {
                } else {
                    // Transaction
                    let mut batch = WriteBatch::default();
                    batch.put(&key, &value);
                    batch.put("term", bincode::serialize(&self.term)?);
                    self.db.write(batch)?;
                }
            }
        }
    }
    async fn remove(&mut self, key: String) -> Result<Option<String>> {
        let ent = LogEntry {
            pid: self.id,
            op: Operation::Remove { key: key.clone() },
        };
        loop {
            self.term += 1;

            let prev = self
                .proposer
                .choose(self.term, serde_json::to_string(&ent)?)
                .await?;
            let prev: LogEntry = serde_json::from_str(&prev)?;
            if prev.pid == self.id {
                if let Operation::Get { key } = prev.op {
                } else {
                    // Transaction
                    let mut batch = WriteBatch::default();
                    batch.delete(&key);
                    batch.put("term", bincode::serialize(&self.term)?);
                    self.db.write(batch)?;
                }
            }
        }
    }
}
