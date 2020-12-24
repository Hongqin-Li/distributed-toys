labrpc::service! {
    service hello {
        fn say(a: i32, x: String) -> String;
    }
}

use hello::{Client, Server, Service};
use labrpc::anyhow::Result;

#[derive(Clone)]
struct MyService {}

#[labrpc::async_trait]
impl Service for MyService {
    async fn say(&mut self, a: i32, x: String) -> Result<String> {
        return Ok(x.to_string());
    }
}

fn main() {
    // let server = Server::new();
}
