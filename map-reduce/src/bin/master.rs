use futures::{future, prelude::*};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::time::SystemTime;

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
    inprogress: Vec<Task>,
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
    async fn hello(self, _: context::Context, name: String) -> String {
        format!(
            "Hello, {}! You are connected from {:?} with files {:?}",
            name,
            self.addr,
            self.context.clone().lock().unwrap().files
        )
    }
    async fn request(self, _: context::Context) -> RequestReply {
        let ctx = self.context.clone();
        let mut c = ctx.lock().unwrap();
        println!("idle {:#?}", c.idle);
        println!("inprogress {:#?}", c.inprogress);
        println!("completed {:#?}", c.completed);

        let task = {
            if let Some(mut t) = c.idle.pop() {
                t.created_at = SystemTime::now();
                c.inprogress.push(t.clone());
                Some(t)
            } else if let Some(t) = c.inprogress.last() {
                // TODO: pop front?
                if let Ok(dt) = SystemTime::now().duration_since(t.created_at) {
                    if dt > c.timeout {
                        println!("Task {:?} timeout, reassigned to other workers.", t);
                        let mut tt = t.clone();
                        tt.created_at = SystemTime::now();
                        Some(tt)
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
                        // Init reduce tasks
                        for i in 0..c.nreduce {
                            c.idle.push(Task {
                                id: i,
                                files: Vec::new(),
                                task: TaskType::Reduce,
                                created_at: SystemTime::now(),
                            });
                        }
                        let t = c.idle.pop().expect("--nreduce should be non-zero");
                        c.inprogress.push(t.clone());
                        Some(t)
                    }
                    TaskType::Reduce => None,
                }
            }
        };

        RequestReply {
            task,
            nmap: c.nmap,
            nreduce: c.nreduce,
        }
    }
    async fn report(self, _: context::Context, id: usize) -> ReportReply {
        let ctx = self.context.clone();
        let mut c = ctx.lock().unwrap();

        let mut resp = None;
        for (i, t) in c.inprogress.iter().enumerate() {
            if t.id == id {
                c.inprogress.remove(i);
                resp = Some(ReportReply { done: true });
                break;
            }
        }
        match resp {
            Some(t) => t,
            None => ReportReply { done: false },
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
                assert!(chunk_size > 0, "input no file or --nmap is set as zero");
                for (id, files) in opt.files.chunks(chunk_size).enumerate() {
                    idle.push(Task {
                        task: TaskType::Map,
                        created_at: SystemTime::now(),
                        id,
                        files: files.into(),
                    });
                }
                idle
            },
            inprogress: Vec::new(),
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
        // Limit channels to 1 per IP.
        .max_channels_per_key(1, |t| t.as_ref().peer_addr().unwrap().ip())
        // serve is generated by the service attribute. It takes as input any type implementing
        // the generated Service trait.
        .map(|channel| channel.respond_with(server.clone().serve()).execute())
        // Max 10 channels.
        .buffer_unordered(10)
        .for_each(|_| async {})
        .await;

    Ok(())
}
