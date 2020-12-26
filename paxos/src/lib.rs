#![deny(missing_docs)]
#![deny(clippy::all)]
//! A basic paxos library.

use serde::{Deserialize, Serialize};

/// Proposal with id and value.
///
/// id is unique for different round of differnt proposer.
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Proposal {
    id: u64,
    value: String,
}

labrpc::service! {
    service acceptor_svc {
        fn prepare(key: u64, pid: u64) -> Option<Proposal>;
        fn accept(key: u64, pid: u64, value: String) -> u64;
    }
}

labrpc::service! {
    service proposer_svc {
        fn choose(key: u64, value: String) -> String;
    }
}

pub use acceptor_svc::{
    Client as AcceptorClient, Server as AcceptorServer, Service as AcceptorService,
};

pub use proposer_svc::{
    Client as ProposerClient, Server as ProposerServer, Service as ProposerService,
};

mod acceptor;
mod persistor;
mod proposer;

/// Util function for testing.
pub mod tests;

pub use acceptor::Acceptor;
pub use persistor::Persistor;
pub use proposer::Proposer;
