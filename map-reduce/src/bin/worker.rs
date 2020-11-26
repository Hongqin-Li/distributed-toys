use atomicwrites::{AllowOverwrite, AtomicFile};
use log::{error, info, trace, warn};
use map_reduce::TaskType;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::time::sleep;

use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::process;
use std::{collections::hash_map::DefaultHasher, io::Write};
use std::{collections::HashMap, fs};
use structopt::StructOpt;
use tarpc::{client, context, tokio_serde::formats::Json};

use map_reduce::app::wc::{map, reduce};
#[derive(Debug, Serialize, Deserialize)]
struct IntermediateFile {
    values: Vec<(String, String)>,
}

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
    let mut timeout = opt.timeout;
    let server_addr = opt.server;
    let server_addr = server_addr
        .parse::<SocketAddr>()
        .unwrap_or_else(|e| panic!(r#"--server_addr value "{}" invalid: {}"#, server_addr, e));

    let client_id = process::id();
    info!("{:?} start", client_id);
    // ServiceClient is generated by the service attribute. It has a constructor `new` that takes a
    // config and any Transport as input.
    let mut client = {
        loop {
            let mut transport = tarpc::serde_transport::tcp::connect(server_addr, Json::default);
            transport.config_mut().max_frame_length(4294967296);
            let r = transport.await;
            if let Ok(rr) = r {
                break map_reduce::ServiceClient::new(client::Config::default(), rr).spawn()?;
            }
            if timeout > 0 {
                timeout = timeout - 1;
                warn!("{} retry: connect to server failed", client_id);
                sleep(Duration::new(1, 0)).await;
            } else {
                error!("{} exit: connection timeout", client_id);
                return Ok(());
            }
        }
    };

    timeout = opt.timeout;
    // The client has an RPC method for each RPC defined in the annotated trait. It takes the same
    // args as defined, with the addition of a Context, which is always the first arg. The Context
    // specifies a deadline and trace information which can be helpful in debugging requests.
    loop {
        let r = client.request(context::current()).await;
        if let Err(rr) = r {
            if timeout >= 0 {
                timeout = timeout - 1;
                warn!("{} retry: connect to server failed", client_id);
                sleep(Duration::new(1, 0)).await;
            } else {
                error!("{} exit: connection timeout", client_id);
                break;
            }
            continue;
        }
        let resp = r.unwrap();
        if let Some(t) = resp.task {
            let nreduce = resp.nreduce;
            let nmap = resp.nmap;
            match t.task {
                TaskType::Map => {
                    for fname in t.files.iter() {
                        let mut cnt = HashMap::<usize, Vec<(String, String)>>::new();
                        let contents = fs::read_to_string(fname).expect("Error on reading file");
                        for (k, v) in map(fname, &contents) {
                            let mut hasher = DefaultHasher::new();
                            k.hash(&mut hasher);
                            let r = (hasher.finish() as usize) % nreduce;
                            let x = cnt.get_mut(&r);
                            if let Some(xv) = x {
                                xv.push((k, v));
                            } else {
                                cnt.insert(r, Vec::from([(k, v)]));
                            }
                        }

                        for (i, values) in cnt.iter() {
                            let path = intermediate_path(t.id, i.to_owned());
                            trace!("{} map: {:?}", client_id, path);
                            let af = AtomicFile::new(path, AllowOverwrite);
                            af.write(|f| {
                                f.write_all(serde_json::to_string(&values).unwrap().as_bytes())
                            })
                            .unwrap();
                            // fs::write(path, serde_json::to_string_pretty(&values).unwrap()).unwrap();
                        }
                    }
                }
                TaskType::Reduce => {
                    let mut map: HashMap<String, Vec<String>> = HashMap::new();
                    for i in 0..nmap {
                        // Reduce
                        let path = intermediate_path(i, t.id);
                        trace!("{} reduce: {:?}", client_id, path);
                        match fs::read_to_string(&path) {
                            Ok(s) => {
                                let values: Vec<(String, String)> =
                                    serde_json::from_str(s.as_ref())?;
                                for (k, v) in values.iter() {
                                    if let Some(vs) = map.get_mut(k) {
                                        vs.push(v.clone());
                                    } else {
                                        map.insert(k.clone(), Vec::from([v.clone()]));
                                    }
                                }
                            }
                            Err(e) => match e.kind() {
                                std::io::ErrorKind::NotFound => {
                                    warn!("reduce {:?} not found, omitted", path);
                                }
                                _ => {
                                    panic!("reduce {:?} error: {:?}", path, e);
                                }
                            }
                        }
                    }
                    let mut result: Vec<(String, String)> = Vec::new();
                    for (k, vs) in map.iter() {
                        result.push((k.clone(), reduce(k, vs)));
                    }
                    result.sort_by(|a, b| a.cmp(b));
                    // Output
                    let path = output_path(t.id);
                    trace!("{} output: {:?}", client_id, path);
                    let af = AtomicFile::new(path, AllowOverwrite);
                    af.write(|f| {
                        let mut s = String::new();
                        f.write_all({
                            for (k, v) in result.iter() {
                                s.push_str(format!("{} {}", k, v).as_ref());
                                s.push('\n');
                            }
                            s.as_ref()
                        })
                    })
                    .unwrap();
                }
                TaskType::Exit => {
                    error!("received exit task");
                }
            }
            match client
                .report(context::current(), t.id, t.task.clone())
                .await
            {
                Ok(r) => {
                    trace!("{} report: {:?} {:?}", client_id, t.id, t.task);
                }
                Err(e) => {
                    warn!("{} report failed: {:?} maybe master finished", client_id, t);
                }
            }
        }
    }
    Ok(())
}
