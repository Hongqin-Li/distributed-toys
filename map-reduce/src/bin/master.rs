use std::path::PathBuf;
use structopt::StructOpt;

use map_reduce::Master;

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
    let m = Master {
        port: opt.port,
        timeout: opt.timeout,
        files: opt.files,
        nmap: opt.nmap,
        nreduce: opt.nreduce,
    };
    m.launch().await?;
    Ok(())
}
