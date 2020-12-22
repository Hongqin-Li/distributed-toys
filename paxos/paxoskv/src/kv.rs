use super::KvService;

use std::{path::{Path, PathBuf}};
use paxos::{Acceptor, AcceptorClient, AcceptorServer, Proposer, ProposerService};
use labrpc::{
    Server, serde::{Serialize, Deserialize},
    serde_json,
};
use rocksdb::{DB, WriteBatch};

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
    Get{key: String},
    Set{key: String, value: String},
    Remove{key: String }
}

pub struct ClusterInfo {
    pub acc_clients: Vec<AcceptorClient>,

}

impl Paxoskv {

    fn new(dir: impl Into<PathBuf>, id: u32, cluster: ClusterInfo) -> Self {
        let dir: PathBuf = dir.into();
        let db_path = dir.join("paxoskv");

        let proposer = Proposer::new(id,cluster.acc_clients);
        let db = DB::open_default(db_path).unwrap();
        let term = db.get("term").unwrap().map_or(0,|v| {
            bincode::deserialize(&v).unwrap()
        });

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
    async fn get_local(&mut self, key: String) -> Option<String> {
        self.db.get(&key).unwrap().map(|v| {
            String::from_utf8(v).unwrap()
        })
    }
    async fn get(&mut self, key: String) -> Option<String> {
        let ent = LogEntry {
            pid: self.id,
            op: Operation::Get {
                key: key.clone(),
            }
        };
        loop {
            self.term += 1;

            let prev = self.proposer.choose(self.term, serde_json::to_string(&ent).unwrap()).await;
            let prev: LogEntry = serde_json::from_str(&prev).unwrap();
            if prev.pid == self.id {
                break self.db.get(&key).unwrap().map(|v| String::from_utf8(v).unwrap())
            }
        }
    }
    async fn set(&mut self, key: String, value: String) -> Option<String> {
        let ent = LogEntry {
            pid: self.id,
            op: Operation::Set {
                key: key.clone(),
                value: value.clone(),
            }
        };
        loop {
            self.term += 1;

            let prev = self.proposer.choose(self.term, serde_json::to_string(&ent).unwrap()).await;
            let prev: LogEntry = serde_json::from_str(&prev).unwrap();
            if prev.pid == self.id {
                if let Operation::Get{key} = prev.op {

                } else {
                    // Transaction
                    let mut batch = WriteBatch::default();
                    batch.put(&key, &value);
                batch.put("term", bincode::serialize(&self.term).unwrap());
                self.db.write(batch).unwrap();
                }
            }
        }
    }
    async fn remove(&mut self, key: String) -> Option<String> {
        let ent = LogEntry {
            pid: self.id,
            op: Operation::Remove {
                key: key.clone(),
            }
        };
        loop {
            self.term += 1;

            let prev = self.proposer.choose(self.term, serde_json::to_string(&ent).unwrap()).await;
            let prev: LogEntry = serde_json::from_str(&prev).unwrap();
            if prev.pid == self.id {
                if let Operation::Get{key} = prev.op {}
                else {
                // Transaction
                let mut batch = WriteBatch::default();
                batch.delete(&key);
                batch.put("term", bincode::serialize(&self.term).unwrap());
                self.db.write(batch).unwrap();
                }
            }
        }
    }
}