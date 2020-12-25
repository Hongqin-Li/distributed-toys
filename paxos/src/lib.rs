#![deny(missing_docs)]
#![deny(clippy::all)]
//! A basic paxos library.

use serde::{Deserialize, Serialize};

/// Proposal with id and value.
///
/// id is unique for different round of differnt proposer.
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Proposal {
    id: u64,
    value: String,
}

labrpc::service! {
    service acceptor_svc {
        fn prepare(key: u64, pid: u64) -> Option<Proposal>;
        fn accept(key: u64, pid: u64, value: String) -> u64;
    }
}

labrpc::service! {
    service proposer_svc {
        fn choose(key: u64, value: String) -> String;
    }
}

pub use acceptor_svc::{
    Client as AcceptorClient, Server as AcceptorServer, Service as AcceptorService,
};

pub use proposer_svc::{
    Client as ProposerClient, Server as ProposerServer, Service as ProposerService,
};

mod acceptor;
mod persistor;
mod proposer;

pub use acceptor::Acceptor;
pub use persistor::Persistor;
pub use proposer::Proposer;

#[cfg(test)]
pub mod tests {

    use crate::{Acceptor, AcceptorClient, AcceptorServer, Proposer, ProposerService};

    use labrpc::{log::info, tokio, tokio::sync::mpsc, Network};
    use rand::distributions::Alphanumeric;
    use rand::Rng;
    use std::convert::TryFrom;
    use std::sync::{Arc, Mutex};
    use tempfile::TempDir;
    use tokio::task::JoinHandle;

    fn random_string(n: usize) -> String {
        let v = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(n)
            .collect::<Vec<u8>>();
        String::from_utf8(v).expect("found invalid UTF-8")
    }

    pub fn acceptor_cluster(
        dir: &TempDir,
        n: u32,
    ) -> (Vec<AcceptorClient>, Vec<JoinHandle<()>>, JoinHandle<()>) {
        let mut net = Network::new();

        let mut acceptor_clients = Vec::new();

        let mut acceptors = Vec::new();

        // Spawn acceptors
        for i in 0..n {
            let id = format!("acc-{}", i);

            let p = dir.path().join(&id);

            let (client, server_routine) = net
                .register_service::<AcceptorServer<Acceptor>, _, _, _>(id, move || {
                    Acceptor::new(p.clone())
                });
            acceptor_clients.push(client);
            acceptors.push(tokio::spawn(server_routine));
        }

        let net_thread = tokio::spawn(async move {
            net.run().await;
        });
        (acceptor_clients, acceptors, net_thread)
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 20)]
    async fn test_single_key() {
        const KEY: u64 = 1;
        const N: u32 = 10;
        const NPROP: u32 = 10;

        env_logger::init();
        let dir = tempfile::TempDir::new().unwrap();

        let mut proposers = Vec::new();

        let (acc_clients, acceptors, net_thread) = acceptor_cluster(&dir, N);

        let (tx, mut rx) = mpsc::channel(usize::try_from(2 * N).unwrap());

        // Spawn proposers
        for i in 0..NPROP {
            let acc_clients = acc_clients.clone();
            let tx = tx.clone();
            proposers.push(tokio::spawn(async move {
                let mut p = Proposer::new(i, acc_clients);
                let k = format!("p[{}]={}", i, random_string(10));
                let s = p.choose(KEY, dbg!(k)).await;
                tx.send(s).await.unwrap();
            }));
        }

        let mut s: Option<String> = None;
        for _ in 0..NPROP {
            let t = rx.recv().await.unwrap().unwrap();
            if let Some(s) = s.clone() {
                assert_eq!(s, t);
            } else {
                s = Some(t);
            }
        }
    }
}
