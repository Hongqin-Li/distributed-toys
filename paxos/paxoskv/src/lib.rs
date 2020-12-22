labrpc::service! {
    service kv_service {
        fn get_local(key: String) -> Option<String>;
        fn get(key: String) -> Option<String>;
        fn set(key: String, value: String) -> Option<String>;
        fn remove(key: String) -> Option<String>;
    }
}

pub use kv_service::{Client as KvClient, Server as KvServer, Service as KvService};

mod kv;
