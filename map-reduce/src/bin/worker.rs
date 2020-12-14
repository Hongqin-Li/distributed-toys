use std::path::PathBuf;

use structopt::StructOpt;

use map_reduce::app::wc;
use map_reduce::Worker;

#[derive(StructOpt, Debug)]
#[structopt(name = env!("CARGO_PKG_NAME"), version = env!("CARGO_PKG_VERSION"), about = env!("CARGO_PKG_DESCRIPTION"), author = env!("CARGO_PKG_AUTHORS"))]
struct Opt {
    #[structopt(short, long)]
    server: String,

    #[structopt(short, long, default_value = "10")]
    timeout: i32,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    let opt = Opt::from_args();
    let w = Worker {
        dir: "target".into(),
        server: opt.server,
        timeout: opt.timeout,
        map: wc::map,
        reduce: wc::reduce,
    };
    w.launch().await;
    Ok(())
}
