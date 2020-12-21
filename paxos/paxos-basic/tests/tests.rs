use paxos_basic::{Acceptor, Proposer, ProposerService};
use paxos_basic::{AcceptorClient, AcceptorServer};
use tokio::{sync::mpsc};
use rand::Rng;
use rand::distributions::Alphanumeric;

fn random_string(n: usize) -> String {
    let v  = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(n)
        .collect::<Vec<u8>>();
    String::from_utf8(v).expect("found invalid UTF-8")
}

#[tokio::test]
async fn test_all() {
    let n = 10;
    let dir = tempfile::TempDir::new().unwrap();

    let mut acceptor_clients = Vec::new();

    let mut acceptors = Vec::new();
    let mut proposers = Vec::new();

    // Spawn acceptors
    for i in 0..n {
        let p = dir.path().join(format!("acc-{}", i));
        let mut acc_server = AcceptorServer::with_service(Acceptor::new(p));
        acceptor_clients.push( AcceptorClient::with_server(acc_server.tx.clone()));

        acceptors.push(tokio::spawn(async move {
            acc_server.run().await;
        }));
    }

    let (tx, mut rx) = mpsc::channel(2*n);
    
    // Spawn proposers
    for i in 0..n {
        let acceptor_clients = acceptor_clients.clone();
        let tx = tx.clone();
        proposers.push(tokio::spawn(async move {
            let mut p = Proposer::new(i as u32, acceptor_clients);
            let k = format!("p[{}]={}", i, random_string(10));
            let s = p.choose(dbg!(k)).await;
            tx.send(s).await.unwrap();
        }));
    }

    let mut s = None;
    for i in 0..n {
        let t = rx.recv().await.unwrap();
        if let Some(s) = s.clone() {
           assert_eq!(s, t);
        } else {
            s = Some(t);
        }
    }
}
