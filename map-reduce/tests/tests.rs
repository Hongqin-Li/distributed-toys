use log::trace;
use map_reduce::{Master, Worker};
use tempfile::TempDir;
use tokio::runtime::Runtime;

use std::collections::HashMap;
use std::{env, fs, path::PathBuf};

fn map(_filename: &PathBuf, contents: &String) -> Vec<(String, String)> {
    // let contents = fs::read_to_string(filename).expect("Something went wrong reading the file");
    let words = contents.split_whitespace();
    let mut cnt = Vec::<(String, String)>::new();
    for w in words {
        cnt.push((w.into(), "1".into()));
    }
    cnt
}

fn reduce(_key: &String, values: &Vec<String>) -> String {
    values.len().to_string()
}

#[test]
fn test_all() {
    pretty_env_logger::init();

    let root_dir = &env::var("CARGO_MANIFEST_DIR").unwrap();
    let mut files = Vec::new();
    for ent in fs::read_dir(PathBuf::from(root_dir).join("../MIT-6.824/src/main")).unwrap() {
        let p = ent.unwrap().path();
        if let Some(name) = p.file_name() {
            if name.to_str().unwrap().to_owned().ends_with("txt") {
                files.push(p);
            }
        }
    }
    println!("{:?}", &files);
    let temp_dir = TempDir::new().expect("unable to create temporary working directory");
    let p = temp_dir.path();
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let mut workers = Vec::new();
        let files = files.clone();
        for _ in 0..10 {
            let dir = p.to_owned();
            workers.push(tokio::task::spawn(async {
                let w = Worker {
                    dir,
                    server: "127.0.0.1:9999".to_owned(),
                    timeout: 10,
                    map: map,
                    reduce: reduce,
                };
                w.launch().await.unwrap();
            }));
        }
        Runtime::new().unwrap().block_on(async {
          let _ = tokio::task::spawn(async {
            Master {
                port: 9999,
                timeout: 10,
                files,
                nmap: 10,
                nreduce: 10,
            }
            .launch()
            .await
        }).await;
      });
    });
    println!("MapReduce finished.");

    // Collection output result
    let result = {
        let mut result = HashMap::<String, String>::new();
        for ent in fs::read_dir(&temp_dir).unwrap() {
            let p = ent.unwrap().path();
            if let Some(name) = p.file_name() {
                if name.to_str().unwrap().to_owned().starts_with("mr-out") {
                    let s = fs::read_to_string(p).unwrap();
                    for l in s.lines() {
                        let kv: Vec<&str> = l.split(' ').collect();
                        assert!(kv.len() == 2);
                        assert!(result.get(kv[0]).is_none());
                        result.insert(kv[0].to_owned(), kv[1].to_owned());
                    }
                }
            }
        }
        result
    };

    // Result of sequential MapReduce
    let seq_result = {
        let mut cnt = HashMap::<String, Vec<String>>::new();
        for fname in files.iter() {
            let contents = fs::read_to_string(fname).expect("Error on reading file");
            for (k, v) in map(fname, &contents) {
                if let Some(xv) = cnt.get_mut(&k) {
                    xv.push(v);
                } else {
                    cnt.insert(k, Vec::from([v]));
                }
            }
        }
        let mut result = HashMap::<String, String>::new();
        for (k, vs) in cnt.iter() {
            let v = reduce(k, vs);
            result.insert(k.to_owned(), v);
        }
        result
    };

    // Check that the result is identical to the result of sequential MapReduce
    assert!(result.len() == seq_result.len());

    for (k, v) in seq_result.iter() {
        assert!(result.get(k).unwrap() == v);
    }
    println!("result: {:?}", &seq_result);
}
