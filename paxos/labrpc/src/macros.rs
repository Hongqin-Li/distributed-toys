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
        $(#[$service_attr])*
        pub mod $svc_name {
            use super::*;

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
                    async fn $method_name(&mut self, $($arg_id : $arg_ty),* ) -> $output;
                )*
            }

            #[derive(Debug, Clone)]
            pub struct Client {
                tx: Sender<(Sender<String>, String)>,
            }

            impl Client {

                $(
                    pub async fn $method_name(&self, $($arg_id : $arg_ty),* ) -> Result<$output> {
                    let req = Request::$method_name {
                        $($arg_id),*
                    };
                    let resp = self.call(serde_json::to_string(&req).unwrap()).await?;
                    let resp: response::$method_name = $crate::serde_json::from_str(&resp)?;
                    Ok(resp.data)
                })*

                pub fn with_server(tx: Sender<(Sender<String>, String)>) -> Self {
                    Self {
                        tx
                    }
                }

                async fn call(&self, req: String) -> Result<String> {
                    let (tx, mut rx) = mpsc::channel(100);
                    let server_tx = self.tx.clone();
                    server_tx.send((tx, req.clone())).await?;
                    if let Some(resp) = rx.recv().await {
                        trace!("req: {}, resp: {}", req, &resp);
                        Ok(resp)
                    } else {
                        Err(anyhow!("unable to receive from server"))
                    }
                }
            }

            #[derive(Debug)]
            pub struct Server<T: Service + Send> {
                svc: T,
                pub tx: Sender<(Sender<String>, String)>,
                rx: Receiver<(Sender<String>, String)>,
            }
            impl<T: Service + Send> Server<T> {
                // pub fn new(server: $crate::Server) -> Self {
                //     Self { server }
                // }

                pub fn with_service(svc: T,) -> Self {
                    let (tx, rx) = mpsc::channel(100);

                    Self {svc, tx, rx }
                }
                pub async fn run(&mut self) {
                    loop {
                        match self.handle().await {
                            Ok(()) => {
                            }
                            Err(e) => {
                                error!("server error: {}", e);
                            }
                        }
                    }
                }
                async fn handle(&mut self) -> Result<()> {

                    match self.rx.recv().await {
                        Some((tx, s)) => {
                            let req: Request = serde_json::from_str(&s)?;
                            match req {
                                $(
                                    Request::$method_name { $($arg_id),* } => {
                                        let data = self.svc.$method_name($($arg_id),* ).await;
                                        let resp = response::$method_name {
                                            data
                                        };
                                        let resp = serde_json::to_string(&resp)?;
                                        trace!("req: {}, resp: {}", &s, &resp);
                                        tx.send(resp).await?;
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
