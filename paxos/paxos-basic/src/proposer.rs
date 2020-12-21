use std::sync::{Arc, Mutex};

use crate::{AcceptorClient, ProposerService};

#[derive(Clone)]
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
    async fn choose(&mut self, value: String) -> String {
        loop {
            self.round += 1;
            let majority = 1 + self.acceptors.len() / 2;
            let pid: u64 = ((self.round as u64) << 32) + (self.id as u64);

            let mut prepared = Vec::new();
            let mut resp = Vec::new();

            for c in self.acceptors.iter() {
                match c.prepare(pid).await {
                    Ok(opt) => {
                        prepared.push(c.clone());
                        if let Some(p) = opt {
                            resp.push(p)
                        }
                    }
                    Err(_) => {}
                }
            }
            if prepared.len() >= majority {
                let mut value = value.clone();
                let mut cur = 0;
                for p in resp.iter() {
                    if p.id > cur {
                        cur = p.id;
                        value = p.value.clone();
                    }
                }

                let mut accepted = 0;
                for c in prepared {
                    if let Ok(_) = { c.accept(pid, value.clone()).await } {
                        accepted += 1;
                    }
                }
                if accepted >= majority {
                    break value;
                }
            }
        }
    }
}
