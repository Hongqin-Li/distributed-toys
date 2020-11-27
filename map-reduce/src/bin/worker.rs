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

fn intermediate_path(map_idx: usize, reduce_idx: usize) -> PathBuf {
    [
        "target",
        format!("mr-{}-{}.json", map_idx, reduce_idx).as_ref(),
    ]
    .iter()
    .collect()
}

fn output_path(reduce_idx: usize) -> PathBuf {
    ["target", format!("mr-out-{}", reduce_idx).as_ref()]
        .iter()
        .collect()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    let opt = Opt::from_args();
    let w = Worker {
        server: opt.server,
        timeout: opt.timeout,
        map: wc::map,
        reduce: wc::reduce,
        intermediate_path,
        output_path,
    };
    w.launch().await;
    Ok(())
}
