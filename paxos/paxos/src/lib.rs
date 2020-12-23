use serde::{Deserialize, Serialize};

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
mod tests {

    use super::*;

    use futures::future::{AbortHandle, Abortable, Aborted};
    use labrpc::{
        log::{error, info, trace},
        tokio,
        tokio::sync::mpsc,
    };
    use rand::distributions::Alphanumeric;
    use rand::Rng;
    use std::convert::TryFrom;

    fn random_string(n: usize) -> String {
        let v = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(n)
            .collect::<Vec<u8>>();
        String::from_utf8(v).expect("found invalid UTF-8")
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 20)]
    async fn test_single_key() {
        const KEY: u64 = 1;
        const N: u32 = 10;
        const NPROP: u32 = 10;

        env_logger::init();
        let dir = tempfile::TempDir::new().unwrap();

        let mut acceptor_clients = Vec::new();

        let mut acceptors = Vec::new();
        let mut proposers = Vec::new();

        // Spawn acceptors
        for i in 0..N {
            let acc_id = format!("acc-{}", i);

            let p = dir.path().join(&acc_id);
            let mut acc_server = AcceptorServer::from_service(Acceptor::new(p));
            let acc_client = AcceptorClient::from_server(&acc_server);

            acceptors.push(tokio::spawn(async move {
                acc_server.run().await;
            }));
            acceptor_clients.push(acc_client);
        }

        let (tx, mut rx) = mpsc::channel(usize::try_from(2 * N).unwrap());

        // Spawn proposers
        for i in 0..NPROP {
            let acceptor_clients = acceptor_clients.clone();
            let tx = tx.clone();
            proposers.push(tokio::spawn(async move {
                let mut p = Proposer::new(i, acceptor_clients);
                let k = format!("p[{}]={}", i, random_string(10));
                let s = p.choose(KEY, dbg!(k)).await;
                tx.send(s).await.unwrap();
            }));
        }

        let mut s: Option<String> = None;
        for i in 0..NPROP {
            let t = rx.recv().await.unwrap().unwrap();
            if let Some(s) = s.clone() {
                assert_eq!(s, t);
            } else {
                s = Some(t);
            }
        }
    }
}
