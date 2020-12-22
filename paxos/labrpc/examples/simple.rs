labrpc::service! {
    service hello {
        fn say(a: i32, x: String) -> String;
    }
}
use hello::{Client, Server, Service};
#[derive(Clone)]
struct MyService {}

#[labrpc::async_trait]
impl Service for MyService {
    async fn say(&mut self, a: i32, x: String) -> String {
        return x.to_string();
    }
}

fn main() {
    // let server = Server::new();
}
