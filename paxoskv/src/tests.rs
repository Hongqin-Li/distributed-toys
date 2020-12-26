use super::*;
use kv::ClusterInfo;
use paxos::{Acceptor, AcceptorClient, AcceptorServer, Proposer, ProposerService};

use labrpc::server::Server;
use labrpc::*;
use tokio::task::JoinHandle;

use crate::kv::Paxoskv;
use paxos::tests::acceptor_cluster;
use std::{convert::TryFrom, path::Path};

/// Create a cluter of KV store for testing.
pub fn kv_cluster(
    dir: &Path,
    n: u32,
    cluster_info: ClusterInfo,
) -> (Vec<KvClient>, Vec<JoinHandle<()>>, JoinHandle<()>) {
    let mut net = Network::new();

    let mut clients = Vec::new();

    let mut servers = Vec::new();

    let server_id = |i| format!("kv-{}", i);

    // Spawn servers.
    for i in 0..n {
        let id = server_id(i);

        let p = dir.join(&id);
        let cluster_info = cluster_info.clone();
        let (client, server_routine) = net
            .register_service::<KvServer<Paxoskv>, _, _, _>(id, move || {
                Paxoskv::new(p.clone(), i, cluster_info.clone())
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
async fn test_set_and_get() {
    const N: u32 = 10;

    env_logger::init();
    let dir = tempfile::TempDir::new().unwrap();

    let (acc_clients, acceptors, acc_net) = acceptor_cluster(dir.path(), N);
    let cluster_info = ClusterInfo { acc_clients };
    let (kv_clients, kvs, kv_net) = kv_cluster(dir.path(), N, cluster_info);

    let get_key = |i| format!("key-{}", i);
    let get_value = |i| format!("value-{}", i);

    let mut setter = Vec::new();
    for i in 0..N {
        let clients = kv_clients.clone();
        setter.push(tokio::spawn(async move {
            let cmd_id = u64::try_from(i).unwrap();
            loop {
                let mut finish = false;
                for c in clients.iter() {
                    if c.set(cmd_id, get_key(i), get_value(i)).await.is_ok() {
                        finish = true;
                        break;
                    }
                }
                if finish {
                    break;
                }
            }
        }));
    }

    for s in setter {
        s.await.expect("setters should not panic");
    }

    for i in 0..N {
        let cmd_id = u64::try_from(i).unwrap();
        loop {
            let mut finish = false;
            for c in kv_clients.iter() {
                if let Ok(opt) = c.get(get_key(i)).await {
                    let v = opt.expect("expect some value saved before");
                    assert!(v == get_value(i));
                    finish = true;
                    break;
                }
            }
            if finish {
                break;
            }
        }
    }
}
