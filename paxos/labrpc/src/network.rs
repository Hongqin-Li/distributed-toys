use std::{
    collections::HashMap,
    panic::{RefUnwindSafe, UnwindSafe},
    pin::Pin,
    sync::{Arc, Mutex},
};

use futures::{Future, FutureExt};
use log::{info, warn};
use tokio::sync::mpsc::{self, Receiver, Sender};

#[derive(Debug)]
pub struct NetworkPackage {
    pub target: String,
    pub reply: Sender<String>,
    pub payload: String,
}

pub struct NetworkService {
    pub id: String,
    pub tx: Sender<(Sender<String>, String)>,
    pub routine: Pin<Box<dyn Future<Output = ()>>>,
}

pub struct Network {
    pub servers: Arc<Mutex<HashMap<String, NetworkService>>>,
    pub tx: Sender<NetworkPackage>,
    rx: Receiver<NetworkPackage>,
}

impl Network {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(100);
        Self {
            servers: Arc::new(Mutex::new(HashMap::new())),
            tx,
            rx,
        }
    }
    pub async fn add_server<F>(
        &self,
        id: String,
        f: impl Fn(Receiver<(Sender<String>, String)>) -> F
            + Send
            + 'static
            + UnwindSafe
            + RefUnwindSafe,
    ) where
        F: Future<Output = ()> + Send + UnwindSafe,
    {
        // {
        //     let servers = self.servers.clone();
        //     let id = id.clone();
        //     Box::pin(async move {
        //         let (tx, rx) = mpsc::channel(100);
        //         {
        //             let mut servers = servers.lock().unwrap();
        //             let s = servers.get_mut(&id).expect("server should be exists");
        //             assert!(s.id == id);
        //             s.tx = tx;
        //         }
        //         async move {
        //             f(rx).await;
        //         }.catch_unwind().await;
        //     })
        // };
        // let (tx, _) = mpsc::channel(100);
        // self.servers.lock().unwrap().insert(
        //     id.clone(),
        //     NetworkService {
        //         id: id.clone(),
        //         tx,
        //         routine,
        //     },
        // );
    }
    // pub fn remove_server(&mut self, id: String) {
    //     self.servers
    //         .lock()
    //         .unwrap()
    //         .remove(&id)
    //         .expect("server should exists when removing");
    // }

    // fn try_restart_server(&mut self) {
    //     let s= self.servers.lock().unwrap();
    //     for (id, svc) in s.iter() {
    //         if svc.handle
    //     }
    // }
    pub async fn run(&mut self) {
        loop {
            let NetworkPackage {
                target,
                reply,
                payload,
            } = self.rx.recv().await.expect("tx won't be dropped by self");
            if let Some(svc) = self.servers.lock().unwrap().get(&target) {
                if svc.tx.send((reply, payload)).await.is_err() {
                    warn!("failed to send to target {}", target);
                }
            } else {
                info!("target not exists {}", target);
            }
        }
    }
}
