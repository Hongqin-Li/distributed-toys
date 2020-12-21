use super::AcceptorService;
use crate::Proposal;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::{
    fs::File,
    io::{Read, Write},
    path::PathBuf,
};

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Acceptor {
    path: PathBuf,
    /// min_pid increases mononitically.
    pid: u64,

    /// accepted.id increases monoitically.
    accepted: Option<Proposal>,
}

impl Acceptor {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        Self::open(path.clone()).map_or_else(
            |e| Self {
                path,
                pid: 0,
                accepted: None,
            },
            |a| a,
        )
    }

    fn open(path: PathBuf) -> Result<Self> {
        let mut f = File::open(&path)?;
        let mut buf = Vec::new();
        f.read_exact(&mut buf)?;
        Ok(bincode::deserialize(&buf)?)
    }
    /// Synchronize self to disk
    ///
    /// This is atomic since Self is smaller than a block size(512B).
    async fn sync2disk(&self) -> Result<()> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&self.path)?;
        file.write_all(bincode::serialize(self)?.as_ref())?;
        Ok(())
    }
}

#[labrpc::async_trait]
impl AcceptorService for Acceptor {
    async fn prepare(&mut self, pid: u64) -> Option<Proposal> {
        if pid > self.pid {
            self.pid = pid;
            self.sync2disk().await.unwrap()
        }
        self.accepted.clone()
    }
    async fn accept(&mut self, pid: u64, value: String) -> u64 {
        if pid == self.pid {
            self.accepted = Some(Proposal { id: pid, value });
            self.sync2disk().await.unwrap();
        } else if pid > self.pid {
            panic!("Unexpected request without prepraration.");
        }
        self.pid
    }
}
