use tokio::sync::mpsc::{Receiver, Sender};

struct Cluster<T> {
    nodes: Vec<ClusterNode<T>>,
}
struct ClusterNode<T> {
    rx: Receiver<(Sender<String>, String)>,
    svc: T,
}

// impl Cluster {
//     fn new(nodes: Vec<(ClusterNode)>) {

//     }
// }
