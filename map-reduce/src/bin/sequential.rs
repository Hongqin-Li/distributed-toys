#![feature(map_first_last)]
use atomicwrites::{AllowOverwrite, AtomicFile};

use log::trace;
use std::path::PathBuf;

use std::{collections::hash_map::DefaultHasher, io::Write};
use std::{
    collections::HashMap,
    fs,
    hash::{Hash, Hasher},
};

use map_reduce::app::wc::{map, reduce};

use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = env!("CARGO_PKG_NAME"), version = env!("CARGO_PKG_VERSION"), about = env!("CARGO_PKG_DESCRIPTION"), author = env!("CARGO_PKG_AUTHORS"))]
struct Opt {
    /// Files to process
    #[structopt(name = "FILE", parse(from_os_str))]
    files: Vec<PathBuf>,

    #[structopt(long, default_value = "3")]
    nmap: usize,

    #[structopt(long, default_value = "10")]
    nreduce: usize,
}

fn output_path(reduce_idx: usize) -> PathBuf {
    ["target", format!("mrs-out-{}", reduce_idx).as_ref()]
        .iter()
        .collect()
}

fn main() {
    pretty_env_logger::init();

    let opt = Opt::from_args();
    let mut cnt = HashMap::<String, Vec<String>>::new();

    for fname in opt.files.iter() {
        let contents = fs::read_to_string(fname).expect("Error on reading file");
        for (k, v) in map(fname, &contents) {
            if let Some(xv) = cnt.get_mut(&k) {
                xv.push(v);
            } else {
                cnt.insert(k, Vec::from([v]));
            }
        }
    }
    let mut result = HashMap::<usize, Vec<(String, String)>>::new();
    for (k, vs) in cnt.iter() {
        let mut hasher = DefaultHasher::new();
        k.hash(&mut hasher);
        let r = (hasher.finish() as usize) % opt.nreduce;
        let v = reduce(k, vs);
        if let Some(vss) = result.get_mut(&r) {
            vss.push((k.clone(), v));
        } else {
            result.insert(r, Vec::from([(k.clone(), v)]));
        }
    }
    for r in 0..opt.nreduce {
        let path = output_path(r.to_owned());
        if let Some(vss) = result.get_mut(&r) {
            vss.sort_by(|a, b| a.cmp(b));
            // Output
            let af = AtomicFile::new(&path, AllowOverwrite);
            af.write(|f| {
                let mut s = String::new();
                f.write_all({
                    for (k, v) in vss.iter() {
                        s.push_str(format!("{} {}", k, v).as_ref());
                        s.push('\n');
                    }
                    s.as_ref()
                })
            })
            .unwrap();
            trace!("output {:?}", path);
        } else {
            let af = AtomicFile::new(&path, AllowOverwrite);
            af.write(|f| f.write_all("".as_ref())).unwrap();
            trace!("output {:?}(empty)", path);
        }
    }
}
