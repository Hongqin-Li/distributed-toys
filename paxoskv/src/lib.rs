labrpc::service! {
    service kv_service {
        fn get_local(key: String) -> Option<String>;
        fn get(key: String) -> Option<String>;
        fn set(cmd_id: u64, key: String, value: String) -> ();
        fn remove(cmd_id: u64, key: String) -> ();
    }
}

pub use kv_service::{Client as KvClient, Server as KvServer, Service as KvService};

mod kv;

#[cfg(test)]
mod tests {
    use super::*;
    use kv::ClusterInfo;
    use paxos::{Acceptor, AcceptorClient, AcceptorServer, Proposer, ProposerService};

    use labrpc::server::Server;
    use labrpc::*;
    use tokio::task::JoinHandle;

    use tempfile::TempDir;

    use crate::kv::Paxoskv;
    use std::convert::TryFrom;

    fn acceptor_cluster(
        dir: &TempDir,
        n: u32,
    ) -> (Vec<AcceptorClient>, Vec<JoinHandle<()>>, JoinHandle<()>) {
        let mut net = Network::new();

        let mut clients = Vec::new();

        let mut servers = Vec::new();

        // Spawn acceptors
        for i in 0..n {
            let id = format!("acc-{}", i);

            let p = dir.path().join(&id);

            let (client, server_routine) = net
                .register_service::<AcceptorServer<Acceptor>, _, _, _>(id, move || {
                    Acceptor::new(p.clone())
                });
            clients.push(client);
            servers.push(tokio::spawn(server_routine));
        }

        let net_thread = tokio::spawn(async move {
            net.run().await;
        });
        (clients, servers, net_thread)
    }

    fn kv_cluster(
        dir: &TempDir,
        n: u32,
        cluster_info: ClusterInfo,
    ) -> (Vec<KvClient>, Vec<JoinHandle<()>>, JoinHandle<()>) {
        let mut net = Network::new();

        let mut clients = Vec::new();

        let mut servers = Vec::new();

        // Spawn acceptors
        for i in 0..n {
            let id = format!("kv-{}", i);

            let p = dir.path().join(&id);
            let cluster_info = cluster_info.clone();
            let (client, server_routine) = net
                .register_service::<KvServer<Paxoskv>, _, _, _>(id, move || {
                    Paxoskv::new(p.clone(), i, cluster_info.clone())
                });
            clients.push(client);
            servers.push(tokio::spawn(server_routine));
        }

        let net_thread = tokio::spawn(async move {
            net.run().await;
        });
        (clients, servers, net_thread)
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 100)]
    async fn test_set_and_get() {
        const N: u32 = 3;

        env_logger::init();
        let dir = tempfile::TempDir::new().unwrap();

        let (acc_clients, acceptors, acc_net) = acceptor_cluster(&dir, N);
        let cluster_info = ClusterInfo { acc_clients };
        let (kv_clients, kvs, kv_net) = kv_cluster(&dir, N, cluster_info);

        let get_key = |i: usize| format!("key-{}", i);
        let get_value = |i: usize| format!("value-{}", i);

        let mut setter = Vec::new();
        for (i, client) in kv_clients.iter().enumerate() {
            let client = client.clone();
            setter.push(tokio::spawn(async move {
                let cmd_id = u64::try_from(i).unwrap();
                loop {
                    if client.set(cmd_id, get_key(i), get_value(i)).await.is_ok() {
                        break;
                    }
                }
            }));
        }

        for s in setter {
            s.await.expect("setters should not panic");
        }

        for (i, client) in kv_clients.iter().enumerate() {
            for (i, client) in kv_clients.iter().enumerate() {
                let cmd_id = u64::try_from(i).unwrap();
                let result = loop {
                    if let Ok(x) = client.get(get_key(i)).await {
                        break x;
                    }
                };
                let result = result
                    .expect("expected some previously saved value, found none");
                assert_eq!(result, get_value(i));
            }
        }
    }
}
