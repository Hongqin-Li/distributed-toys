#![feature(map_first_last)]

use std::path::PathBuf;
use std::time::SystemTime;
use tarpc::serde::{Deserialize, Serialize};

pub mod app;
mod master;
mod worker;
pub use master::Master;
pub use worker::Worker;

#[tarpc::service]
pub trait Service {
    async fn request() -> RequestReply;
    async fn report(id: usize, task: TaskType) -> ReportReply;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskType {
    Map,
    Reduce,
    Exit,
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
