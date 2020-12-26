use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use anyhow::Result;
use futures::Future;
use log::{info, warn};
use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::{client::Client, server::Server};

pub fn is_send<T: Send>(x: &T) {}

#[derive(Debug, Clone)]
pub struct NetworkPackage {
    pub to: String,
    pub reply: Sender<String>,
    pub data: String,
}

pub struct Network {
    pub tx: Sender<NetworkPackage>,
    rx: Receiver<NetworkPackage>,
    pub nodes: Arc<Mutex<HashMap<String, Sender<NetworkPackage>>>>,
}

impl Network {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(100);
        Self {
            tx,
            rx,
            nodes: Arc::new(Mutex::new(HashMap::default())),
        }
    }

    pub fn register_service<S, C, F, V>(&self, id: String, f: F) -> (C, impl Future<Output = ()>)
    where
        F: Fn() -> V,
        S: Server<Service = V> + Send + 'static,
        C: Client,
    {
        let acc_client = C::from_server(id.clone(), self.tx.clone());
        let nodes = self.nodes.clone();
        (acc_client, async move {
            loop {
                let mut server = S::from_service(f());
                nodes
                    .lock()
                    .unwrap()
                    .insert(id.clone(), server.client_chan());
                if let Ok(_) = server.run().await {
                    break;
                } else {
                    info!("server restart");
                }
            }
        })
    }

    pub async fn run(&mut self) {
        loop {
            let p = self
                .rx
                .recv()
                .await
                .expect("sender cannot be dropped by itself");
            let node = {
                let x = self.nodes.lock().unwrap();
                x.get(&p.to).map(|x| x.clone())
            };

            if let Some(x) = node {
                if x.send(p).await.is_err() {
                    warn!("send to node failed, dropped");
                }
            } else {
                warn!("node not found");
            }
        }
    }
}
