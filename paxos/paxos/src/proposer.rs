use crate::{AcceptorClient, ProposerService};
use labrpc::{
    anyhow::Result,
    log::{error, trace},
    tokio,
};
use rand::Rng;
use std::sync::{Arc, Mutex};
use std::{thread, time};

#[derive(Debug, Clone)]
pub struct Proposer {
    acceptors: Vec<AcceptorClient>,
    id: u32,
    round: u32,
}

impl Proposer {
    pub fn new(id: u32, acceptors: Vec<AcceptorClient>) -> Self {
        Self {
            id,
            acceptors,
            round: 0,
        }
    }
}

#[labrpc::async_trait]
impl ProposerService for Proposer {
    async fn choose(&mut self, key: u64, value: String) -> Result<String> {
        loop {
            self.round += 1;
            let majority = 1 + self.acceptors.len() / 2;
            let pid: u64 = ((self.round as u64) << 32) + (self.id as u64);

            let mut prepared = Vec::new();
            let mut resp = Vec::new();

            for c in self.acceptors.iter() {
                match c.prepare(key, pid).await {
                    Ok(opt) => {
                        prepared.push(c);
                        if let Some(p) = opt {
                            resp.push(p)
                        }
                    }
                    Err(e) => {
                        error!("choose client error: {}", e);
                    }
                }
            }
            if prepared.len() >= majority {
                let value = {
                    if let Some(latest) = resp.iter().max_by_key(|p| p.id) {
                        latest.value.clone()
                    } else {
                        value.clone()
                    }
                };

                let mut accepted = 0;
                for c in prepared {
                    if let Ok(aid) = c.accept(key, pid, value.clone()).await {
                        if (aid == pid) {
                            accepted += 1;
                            if accepted >= majority {
                                return Ok(value);
                            }
                        }
                    }
                }
            }
            let dt: u64 = rand::thread_rng().gen_range(10..2000);
            tokio::time::sleep(time::Duration::from_millis(dbg!(dt))).await;
        }
    }
}
