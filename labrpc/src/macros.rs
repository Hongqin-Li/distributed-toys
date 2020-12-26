#[macro_export]
macro_rules! random_error {
    ($prob:tt) => {
        #[cfg(debug_assertions)]
        {
            use $crate::rand::Rng;
            let mut rng = $crate::rand::thread_rng();
            let x: f32 = rng.gen_range(0.0..1.0);
            if x < $prob {
                $crate::log::error!("random error");
                return Err($crate::anyhow::anyhow!("random error"));
            }
        }
    };
}

#[macro_export]
macro_rules! service {
    () => {
        compile_error!("empty service is not allowed");
    };
    (
        $(#[$service_attr:meta])*
        service $svc_name:ident {
            $(
                $(#[$method_attr:meta])*
                fn $method_name:ident($($arg_id:ident: $arg_ty:ty),*) -> $output:ty;
            )*
        }
    ) => {
        #[allow(missing_docs)]
        $(#[$service_attr])*
        pub mod $svc_name {
            use super::*;

            use $crate::network::{Network, NetworkPackage};
            use $crate::{server, client};

            use $crate::tokio::sync::mpsc::{self, Sender, Receiver};
            use $crate::serde_json::{self, Value};
            use $crate::serde::{Serialize, Deserialize};
            use $crate::anyhow::{Result, anyhow};
            use $crate::async_trait;
            use $crate::log::{error, trace};


            #[derive(Debug, Deserialize, Serialize)]
            pub enum Request {
                $(
                    #[allow(non_camel_case_types)]
                    $method_name {  $($arg_id : $arg_ty),* }
                ),*
            }

            mod response {
                use super::*;
                $(
                    #[derive(Deserialize, Serialize)]
                    #[allow(non_camel_case_types)]
                    pub struct $method_name {
                        // TODO: status field
                        pub data: $output
                    }
                )*
            }

            #[async_trait]
            pub trait Service: Send + 'static {
                $(
                    $(#[$method_attr])*
                    async fn $method_name(&mut self, $($arg_id : $arg_ty),* ) -> Result<$output>;
                )*
            }

            #[derive(Debug, Clone)]
            pub struct Client {
                server_id: String,
                tx: Sender<NetworkPackage>,
            }

            impl Client {

                $(
                    pub async fn $method_name(&self, $($arg_id : $arg_ty),* ) -> Result<$output> {
                        let req = Request::$method_name {
                            $($arg_id),*
                        };
                        let resp = self.call(serde_json::to_string(&req)?).await?;
                        let resp: response::$method_name = $crate::serde_json::from_str(&resp)?;
                        Ok(resp.data)
                    }
                )*

                pub async fn call(&self, req: String) -> Result<String> {
                    let (tx, mut rx) = mpsc::channel(100);
                    self.tx.send(NetworkPackage{to: self.server_id.clone(), reply: tx, data: req.clone()}).await?;
                    if let Some(resp) = rx.recv().await {
                        trace!("req: {}, resp: {}", req, &resp);
                        Ok(resp)
                    } else {
                        Err(anyhow!("unable to receive from server"))
                    }
                }
            }

            impl client::Client for Client {
                fn from_server(server_id: String, net_tx: Sender<NetworkPackage>) -> Self {
                    Self {
                        server_id,
                        tx: net_tx,
                    }
                }
            }

            #[derive(Debug)]
            pub struct Server<T: Service + Send> {
                svc: T,
                tx: Sender<NetworkPackage>,
                rx: Receiver<NetworkPackage>,
            }

            #[async_trait]
            impl<T: Service + Send> server::Server for Server<T> {
                type Service = T;

                fn from_service(svc: Self::Service) -> Self {
                    let (tx, rx) = mpsc::channel(100);
                    Self {svc, tx, rx}
                }

                fn client_chan(&self) -> Sender<NetworkPackage> {
                    return self.tx.clone();
                }

                async fn handle(&mut self) -> Result<()> {

                    match self.rx.recv().await {
                        Some(NetworkPackage{to, reply, data}) => {
                            trace!("handle recv: {}", &data);
                            let req: Request = serde_json::from_str(&data)?;
                            match req {
                                $(
                                    Request::$method_name { $($arg_id),* } => {
                                        let data = self.svc.$method_name($($arg_id),* ).await?;
                                        let resp = response::$method_name {
                                            data
                                        };
                                        let resp = serde_json::to_string(&resp)?;
                                        trace!("handle send: {}", &resp);
                                        reply.send(resp).await?;
                                        Ok(())
                                    }
                                )*
                            }
                        }
                        None => {Err(anyhow!("expected sender"))}
                    }
                }
            }
        }
    };
}
