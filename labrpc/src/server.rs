use crate::network::NetworkPackage;
use anyhow::Result;
use tokio::sync::mpsc::Sender;

#[async_trait::async_trait]
pub trait Server {
    type Service;
    fn from_service(svc: Self::Service) -> Self;
    fn client_chan(&self) -> Sender<NetworkPackage>;
    async fn handle(&mut self) -> Result<()>;
    async fn run(&mut self) -> Result<()> {
        loop {
            self.handle().await?;
        }
    }
}
