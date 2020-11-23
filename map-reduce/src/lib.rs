use std::path::PathBuf;
use std::time::SystemTime;
use tarpc::serde::{Deserialize, Serialize};

pub mod app;

#[tarpc::service]
pub trait Service {
    /// Returns a greeting for name.
    async fn hello(name: String) -> String;
    async fn request() -> RequestReply;
    async fn report(id: usize) -> ReportReply;
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskType {
    Map,
    Reduce,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub task: TaskType,
    pub created_at: SystemTime,
    pub id: usize,
    pub files: Vec<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RequestReply {
    pub task: Option<Task>,
    pub nmap: usize,
    pub nreduce: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReportReply {
    pub done: bool,
}

pub struct KeyValue {
    key: String,
    value: String,
}
