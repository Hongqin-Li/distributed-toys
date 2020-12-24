use super::KvService;

use labrpc::{
    anyhow::Result,
    random_error,
    serde::{Deserialize, Serialize},
    serde_json,
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

#[derive(Clone)]
pub struct ClusterInfo {
    pub acc_clients: Vec<AcceptorClient>,
}

impl Paxoskv {
    pub fn new(path: impl AsRef<Path>, id: u32, cluster: ClusterInfo) -> Self {
        let proposer = Proposer::new(id, cluster.acc_clients);
        let db = DB::open_default(path).unwrap();
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
    async fn get(&mut self, mut key: String) -> Result<Option<String>> {
        key = key + "-"; // Keys not end with '-' are used internally.

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
    /// `command_id` is used for eliminating duplication when client retries.
    async fn set(&mut self, cmd_id: u64, mut key: String, value: String) -> Result<()> {
        let cmd_key = format!("{}-command", cmd_id);
        if self.db.get(&cmd_key)?.is_some() {
            return Ok(())
        }

        key = key + "-";

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
                    batch.put(&cmd_key, "x");
                    self.db.write(batch)?;
                    return Ok(());
                }
            }
        }
    }
    async fn remove(&mut self, cmd_id: u64, mut key: String) -> Result<()> {
        let cmd_key = format!("{}-command", cmd_id);
        if self.db.get(&cmd_key)?.is_some() {
            return Ok(())
        }

        key = key + "-";
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
                    batch.put(&cmd_key, "x");
                    self.db.write(batch)?;
                    return Ok(())
                }
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use std::vec;

    use super::*;
    use tempfile::TempDir;


    #[test]
    fn test_new() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("tmp");
        let kv = Paxoskv::new(path, 1, ClusterInfo {acc_clients: Vec::new()});
    }
}