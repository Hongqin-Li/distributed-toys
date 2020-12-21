use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Proposal {
    id: u64,
    value: String,
}

labrpc::service! {
    service acceptor_svc {
        fn prepare(pid: u64) -> Option<Proposal>;
        fn accept(pid: u64, value: String) -> u64;
    }
}

labrpc::service! {
    service proposer_svc {
        fn choose(value: String) -> String;
    }
}

pub use acceptor_svc::{
    Client as AcceptorClient, Server as AcceptorServer, Service as AcceptorService,
};

pub use proposer_svc::{
    Client as ProposerClient, Server as ProposerServer, Service as ProposerService,
};

mod acceptor;
mod proposer;

pub use acceptor::Acceptor;
pub use proposer::Proposer;
