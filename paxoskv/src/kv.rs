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

/// KV service instance.
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
    id: u64,
    op: Operation,
}

#[derive(Clone, Serialize, Deserialize)]
enum Operation {
    Get { key: String },
    Set { key: String, value: String },
    Remove { key: String },
}
/// Cluter information shared among each server.
#[derive(Clone)]
pub struct ClusterInfo {
    /// Clients of all acceptors of the cluster.
    pub acc_clients: Vec<AcceptorClient>,
}

impl Paxoskv {
    /// Create a new service instance by path to local DB, id and cluster information.
    ///
    /// The id should be unique between different KV services.
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

    /// Preprocess user key before feeding into database.
    fn user_key(key: String) -> String {
        key + "-"
    }

    /// Append one entry to log with `cmd_id`.
    ///
    /// `cmd_id` is used for eliminating duplication when client retries.
    /// Uniqueness should be guaranteed between different client requests.
    async fn append(&mut self, cmd_id: u64, op: Operation) -> Result<()> {
        let cmd_key = format!("cmd:{}", cmd_id);

        if self.db.get(&cmd_key)?.is_some() {
            return Ok(());
        }

        let ent = LogEntry { id: cmd_id, op };

        loop {
            self.term += 1;
            let cur = self
                .proposer
                .choose(self.term, serde_json::to_string(&ent)?)
                .await?;
            let cur: LogEntry = serde_json::from_str(&cur)?;
            match cur.op {
                Operation::Get { key } => {
                    assert!(key.ends_with("-"));
                }
                Operation::Set { key, value } => {
                    assert!(key.ends_with("-"));
                    let mut batch = WriteBatch::default();
                    batch.put(&key, &value);
                    batch.put("term", bincode::serialize(&self.term)?);
                    batch.put(&cmd_key, "done");
                    self.db.write(batch)?;
                }
                Operation::Remove { key } => {
                    assert!(key.ends_with("-"));

                    let mut batch = WriteBatch::default();
                    batch.delete(&key);
                    batch.put("term", bincode::serialize(&self.term)?);
                    batch.put(&cmd_key, "done");
                    self.db.write(batch)?;
                }
            }
            if cur.id == ent.id {
                break Ok(());
            }
        }
    }
}

#[labrpc::async_trait]
impl KvService for Paxoskv {
    async fn get_local(&mut self, key: String) -> Result<Option<String>> {
        let key = Self::user_key(key);
        if let Some(v) = self.db.get(&key)? {
            Ok(Some(String::from_utf8(v)?))
        } else {
            Ok(None)
        }
    }
    async fn get(&mut self, key: String) -> Result<Option<String>> {
        let key = Self::user_key(key);
        self.append(0xFFFF_FFFF_FFFF_FFFF, Operation::Get { key: key.clone() })
            .await?;
        let opt = self.db.get(&key)?;
        if let Some(v) = opt {
            Ok(Some(String::from_utf8(v)?))
        } else {
            Ok(None)
        }
    }
    async fn set(&mut self, cmd_id: u64, key: String, value: String) -> Result<()> {
        let key = Self::user_key(key);

        let op = Operation::Set { key, value };
        self.append(cmd_id, op).await
    }
    async fn remove(&mut self, cmd_id: u64, key: String) -> Result<()> {
        let key = Self::user_key(key);

        let op = Operation::Remove { key };
        self.append(cmd_id, op).await
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
        let kv = Paxoskv::new(
            path,
            1,
            ClusterInfo {
                acc_clients: Vec::new(),
            },
        );
    }
}
