use anyhow::Result;
use log::error;
use tokio::sync::mpsc::{Receiver, Sender};

#[async_trait::async_trait]
pub trait Server<T: Send + 'static> {
    fn with_service(svc: T, rx: Receiver<(Sender<String>, String)>) -> Box<Self>;
    // May panic
    async fn handle(&mut self) -> Result<()>;

    fn restart(&mut self) {}

    async fn run(&mut self) {
        loop {
            match self.handle().await {
                Ok(()) => {}
                Err(e) => {
                    error!("server error: {}", e);
                }
            }
        }
    }
}
