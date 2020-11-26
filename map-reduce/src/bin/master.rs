#![feature(map_first_last)]

use futures::{future, prelude::*};
use log::{info, trace};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::time::SystemTime;
use std::{collections::BTreeMap, process};
use tokio::time::sleep;

use map_reduce::{ReportReply, RequestReply, Service};

use structopt::StructOpt;
use tarpc::{
    context,
    server::{self, Channel, Handler},
    tokio_serde::formats::Json,
};

use map_reduce::{Task, TaskType};

#[derive(Debug, Clone)]
struct ServerContext {
    files: Vec<PathBuf>,
    nmap: usize,
    nreduce: usize,
    timeout: Duration,

    state: TaskType,

    idle: Vec<Task>,
    inprogress: BTreeMap<usize, Task>,
    completed: Vec<Task>,
}

// This is the type that implements the generated Service trait. It is the business logic
// and is used to start the server.
#[derive(Debug, Clone)]
struct MapReduceServer {
    addr: SocketAddr,
    context: Arc<Mutex<ServerContext>>,
}

#[tarpc::server]
impl Service for MapReduceServer {
    async fn request(self, _: context::Context) -> RequestReply {
        let ctx = self.context.clone();
        let mut c = ctx.lock().unwrap();
        let timeout = c.timeout;

        let task = {
            if let Some(mut t) = c.idle.pop() {
                t.created_at = SystemTime::now();
                c.inprogress.insert(t.id, t.clone());
                Some(t)
            } else if let Some(mut t) = c.inprogress.first_entry() {
                if let Ok(dt) = SystemTime::now().duration_since(t.get().created_at) {
                    if dt > timeout {
                        println!("Task timeout, reassigned to other workers: {:?}", t);
                        t.get_mut().created_at = SystemTime::now();
                        Some(t.get().clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                // All completed
                match c.state {
                    TaskType::Map => {
                        c.state = TaskType::Reduce;
                        let nmap = c.nmap;
                        // Init reduce tasks
                        for i in 0..c.nreduce {
                            c.idle.push(Task {
                                // Map tasks are [0, nmap), reduce tasks are [nmap, nmap + nreduce)
                                id: i,
                                files: Vec::new(),
                                task: TaskType::Reduce,
                                created_at: SystemTime::now(),
                            });
                        }
                        let t = c.idle.pop().expect("--nreduce should be non-zero");
                        c.inprogress.insert(t.id, t.clone());
                        Some(t)
                    }
                    TaskType::Reduce => None,
                    TaskType::Exit => {
                        info!("exiting");
                        process::exit(0);
                    }
                }
            }
        };

        RequestReply {
            task,
            nmap: c.nmap,
            nreduce: c.nreduce,
        }
    }
    async fn report(self, _: context::Context, id: usize, task: TaskType) -> ReportReply {
        let ctx = self.context.clone();
        let mut c = ctx.lock().unwrap();
        if task == c.state {
            if let Some(t) = c.inprogress.remove(&id) {
                c.completed.push(t);
                if TaskType::Reduce == c.state && c.idle.len() == 0 && c.inprogress.len() == 0 {
                    c.state = TaskType::Exit;
                }
                ReportReply { done: true }
            } else {
                ReportReply { done: false }
            }
        } else {
            ReportReply { done: false }
        }
    }
}

#[derive(StructOpt, Debug)]
#[structopt(name = env!("CARGO_PKG_NAME"), version = env!("CARGO_PKG_VERSION"), about = env!("CARGO_PKG_DESCRIPTION"), author = env!("CARGO_PKG_AUTHORS"))]
struct Opt {
    /// Port to start master server
    #[structopt(short, long)]
    port: u16,

    /// Timeout in seconds used for communication with workers
    #[structopt(short, long, default_value = "10")]
    timeout: u64,

    /// Files to process
    #[structopt(name = "FILE", parse(from_os_str))]
    files: Vec<PathBuf>,

    #[structopt(long, default_value = "3")]
    nmap: usize,

    #[structopt(long, default_value = "10")]
    nreduce: usize,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    let opt = Opt::from_args();

    let server_addr = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), opt.port);
    let server = MapReduceServer {
        addr: server_addr.into(),
        context: Arc::new(Mutex::new(ServerContext {
            files: opt.files.clone(),
            timeout: Duration::new(opt.timeout, 0),
            nmap: opt.nmap,
            nreduce: opt.nreduce,
            state: TaskType::Map,
            idle: {
                let mut idle = Vec::new();
                let chunk_size = (opt.files.len() + opt.nmap - 1) / opt.nmap;
                trace!("chunk_size = {}", chunk_size);
                assert!(chunk_size > 0, "input no file or --nmap is set as zero");
                for (id, files) in opt.files.chunks(chunk_size).enumerate() {
                    assert!(id < opt.nmap);
                    idle.push(Task {
                        task: TaskType::Map,
                        created_at: SystemTime::now(),
                        id,
                        files: files.into(),
                    });
                }
                idle
            },
            inprogress: BTreeMap::new(),
            completed: Vec::new(),
        })),
    };
    // JSON transport is provided by the json_transport tarpc module. It makes it easy
    // to start up a serde-powered json serialization strategy over TCP.
    let mut listener = tarpc::serde_transport::tcp::listen(&server_addr, Json::default).await?;
    listener.config_mut().max_frame_length(4294967296);
    listener
        // Ignore accept errors.
        .filter_map(|r| future::ready(r.ok()))
        .map(server::BaseChannel::with_defaults)
        // Limit channels to 10 per IP.
        .max_channels_per_key(10, |t| t.as_ref().peer_addr().unwrap().ip())
        // serve is generated by the service attribute. It takes as input any type implementing
        // the generated Service trait.
        .map(|channel| channel.respond_with(server.clone().serve()).execute())
        // Max 10 channels.
        .buffer_unordered(10)
        .for_each(|_| async {})
        .await;

    Ok(())
}
