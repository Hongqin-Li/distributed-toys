use tokio::sync::mpsc::Sender;

use crate::network::NetworkPackage;

pub trait Client {
    fn from_server(server_id: String, net_tx: Sender<NetworkPackage>) -> Self;
}
