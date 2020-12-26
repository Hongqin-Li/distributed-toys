use super::KvClient;

/// Client for a KV cluster
pub struct Client {
    clients: Vec<KvClient>,
}

impl Client {
    /// Create a new client from a set of clients of kv
    pub fn new(clients: Vec<KvClient>) -> Self {
        Self { clients }
    }
}
