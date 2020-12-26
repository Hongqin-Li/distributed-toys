use criterion::BenchmarkId;
use criterion::Criterion;
use criterion::{criterion_group, criterion_main};

use labrpc::{
    futures::executor::block_on,
    tokio::{self, runtime::Builder, task, time::Instant},
};

use std::{convert::TryFrom, path::Path};

use paxos::tests::acceptor_cluster;
use paxoskv::{kv::ClusterInfo, tests::kv_cluster};

fn bench_set(c: &mut Criterion) {
    env_logger::init();
    const N: u32 = 11;
    const NQUERIES: u32 = 10000;

    c.bench_function(&format!("{} set op with {} nodes", NQUERIES, N), |b| {
        b.iter_custom(|iters| {
            let rt = Builder::new_multi_thread()
                .worker_threads(30)
                .enable_all()
                .build()
                .unwrap();

            rt.block_on(async {
                let dir = tempfile::TempDir::new().unwrap();

                let (acc_clients, acceptors, acc_net) = acceptor_cluster(dir.path(), N);
                let cluster_info = ClusterInfo { acc_clients };
                let (kv_clients, kvs, kv_net) = kv_cluster(dir.path(), N, cluster_info);

                let get_key = |i| format!("key-{}", i);
                let get_value = |i| format!("value-{}", i);

                let mut setter = Vec::new();

                // Warm up
                let clients = kv_clients.clone();
                let c = clients.first().expect("cluster should not be empty");
                loop {
                    if let Ok(opt) = c.get("none".to_string()).await {
                        assert!(opt.is_none());
                        break;
                    }
                }
                println!("start iters: {}, #node: {}, #query: {}", iters, N, NQUERIES);

                let start = Instant::now();

                for _ in 0..iters {
                    for i in 0..NQUERIES {
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
                }

                for s in setter {
                    s.await.expect("setters should not panic");
                }

                start.elapsed()
            })
        });
    });
}

criterion_group!(
    name = benches;
    // This can be any expression that returns a `Criterion` object.
    config = Criterion::default().sample_size(10);
    targets = bench_set,
);
criterion_main!(benches);
