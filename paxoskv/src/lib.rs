#![deny(missing_docs)]
#![deny(clippy::all)]
//! Distributed KV Store based on Paxos.

labrpc::service! {
    service kv_service {
        fn get_local(key: String) -> Option<String>;
        fn get(key: String) -> Option<String>;
        fn set(cmd_id: u64, key: String, value: String) -> ();
        fn remove(cmd_id: u64, key: String) -> ();
    }
}

pub use kv_service::{Client as KvClient, Server as KvServer, Service as KvService};

/// KV Store Server.
pub mod kv;

/// KV Store Client.
pub mod client;

/// Util function for testing.
pub mod tests;
