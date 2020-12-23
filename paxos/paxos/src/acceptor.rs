use std::path::Path;

use super::AcceptorService;
use crate::Persistor;
use crate::Proposal;
use labrpc::{anyhow::Result, random_error};
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
    async fn prepare(&mut self, key: u64, pid: u64) -> Result<Option<Proposal>> {
        let key_pid = format!("{}:pid", key);
        let key_accepted = format!("{}:accepted", key);
        let newer = self.persistor.get(&key_pid)?.map_or_else(
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
            random_error(0.1)?;
            self.persistor.set(&key_pid, &pid)?;
        };

        random_error(0.1)?;
        Ok(self.persistor.get(&key_accepted)?)
    }
    async fn accept(&mut self, key: u64, pid: u64, value: String) -> Result<u64> {
        let key_pid = format!("{}:pid", key);
        let prev_pid = self.persistor.get(&key_pid)?.expect("unprepared");
        if pid == prev_pid {
            let key_accepted = format!("{}:accepted", key);
            
            random_error(0.1)?;

            self.persistor
                .set(&key_accepted, &Proposal { id: pid, value })?;
        } else if pid > prev_pid {
            panic!("Unexpected request without prepraration.");
        }
        random_error(0.1)?;

        Ok(prev_pid)
    }
}
