#![feature(async_closure)]

// pub use async_trait::async_trait;
pub mod client;
mod cluster;
mod macros;
// pub mod network;
mod server;

pub use anyhow;
pub use async_trait::async_trait;
pub use futures;
pub use log;
pub use serde;
pub use serde_json;
pub use tokio;

pub use server::Server;

use rand::Rng;

// pub use labrpc_macro::server;
// pub use labrpc_macro::service;

pub fn random_error(prob: f32) -> anyhow::Result<()> {
    let mut rng = rand::thread_rng();
    let x: f32 = rng.gen_range(0.0..1.0);
    if x < prob {
        Err(anyhow::anyhow!("random error"))
    } else {
        Ok(())
    }
}
