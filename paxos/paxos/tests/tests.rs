use labrpc::{
    log::{error, info, trace},
    tokio,
    tokio::sync::mpsc,
};
use paxos::{Acceptor, Proposer, ProposerService};
use paxos::{AcceptorClient, AcceptorServer};
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

#[tokio::test(flavor = "multi_thread", worker_threads = 100)]
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
        let p = dir.path().join(format!("acc-{}", i));
        let mut acc_server = AcceptorServer::with_service(Acceptor::new(p));
        acceptor_clients.push(AcceptorClient::with_server(acc_server.tx.clone()));

        acceptors.push(tokio::spawn(async move {
            acc_server.run().await;
        }));
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

    let mut s = None;
    for i in 0..NPROP {
        let t = rx.recv().await.unwrap();
        if let Some(s) = s.clone() {
            assert_eq!(s, t);
        } else {
            s = Some(t);
        }
    }
}
