use std::path::PathBuf;

pub fn map(_filename: &PathBuf, contents: &String) -> Vec<(String, String)> {
    // let contents = fs::read_to_string(filename).expect("Something went wrong reading the file");
    let words = contents.split_whitespace();
    let mut cnt = Vec::<(String, String)>::new();
    for w in words {
        cnt.push((w.into(), "1".into()));
    }
    cnt
}

pub fn reduce(_key: &String, values: &Vec<String>) -> String {
    values.len().to_string()
}
