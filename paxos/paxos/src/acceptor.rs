use std::path::Path;

use super::AcceptorService;
use crate::Persistor;
use crate::Proposal;
use serde::Serialize;

pub struct Acceptor {
    persistor: Persistor,
}

impl Acceptor {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            persistor: Persistor::new(path),
        }
    }
}

#[labrpc::async_trait]
impl AcceptorService for Acceptor {
    async fn prepare(&mut self, key: u64, pid: u64) -> Option<Proposal> {
        let key_pid = format!("{}:pid", key);
        let key_accepted = format!("{}:accepted", key);
        let newer = self.persistor.get(&key_pid).map_or_else(
            || Some(pid),
            |prev| {
                if pid < prev {
                    None
                } else {
                    Some(pid)
                }
            },
        );
        if let Some(pid) = newer {
            self.persistor.set(&key_pid, &pid);
        };

        self.persistor.get(&key_accepted)
    }
    async fn accept(&mut self, key: u64, pid: u64, value: String) -> u64 {
        let key_pid = format!("{}:pid", key);
        let prev_pid = self.persistor.get(&key_pid).expect("unprepared");
        if pid == prev_pid {
            let key_accepted = format!("{}:accepted", key);
            self.persistor
                .set(&key_accepted, &Proposal { id: pid, value });
        } else if pid > prev_pid {
            panic!("Unexpected request without prepraration.");
        }
        prev_pid
    }
}
