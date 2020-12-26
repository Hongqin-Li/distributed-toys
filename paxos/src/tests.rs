use crate::{Acceptor, AcceptorClient, AcceptorServer, Proposer, ProposerService};

use labrpc::{log::info, tokio, tokio::sync::mpsc, Network};
use rand::distributions::Alphanumeric;
use rand::Rng;
use std::sync::{Arc, Mutex};
use std::{convert::TryFrom, path::Path};
use tokio::task::JoinHandle;

/// Create random string of length n.
pub fn random_string(n: usize) -> String {
    let v = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(n)
        .collect::<Vec<u8>>();
    String::from_utf8(v).expect("found invalid UTF-8")
}

/// Create a cluster of acceptors for testing.
pub fn acceptor_cluster(
    dir: &Path,
    n: u32,
) -> (Vec<AcceptorClient>, Vec<JoinHandle<()>>, JoinHandle<()>) {
    let mut net = Network::new();
    let mut clients = Vec::new();
    let mut servers = Vec::new();

    let server_id = |i| format!("acc-{}", i);

    // Spawn servers.
    for i in 0..n {
        let id = server_id(i);

        let p = dir.join(&id);

        let (client, server_routine) = net
            .register_service::<AcceptorServer<Acceptor>, _, _, _>(id, move || {
                Acceptor::new(p.clone())
            });
        clients.push(client);
        servers.push(tokio::spawn(server_routine));
    }

    // Wait until servers finish registration.
    let nodes = net.nodes.clone();
    for i in 0..n {
        let id = server_id(i);
        loop {
            if nodes.clone().lock().unwrap().get(&id).is_some() {
                break;
            }
        }
    }

    let net_thread = tokio::spawn(async move {
        net.run().await;
    });

    (clients, servers, net_thread)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 20)]
async fn test_single_key() {
    const KEY: u64 = 1;
    const N: u32 = 10;
    const NPROP: u32 = 10;

    env_logger::init();
    let dir = tempfile::TempDir::new().unwrap();

    let mut proposers = Vec::new();

    let (acc_clients, acceptors, net_thread) = acceptor_cluster(dir.path(), N);

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
