macro_rules! arglist {
    () => {};
    (
        $id:ident; $type:ty
    ) => {
        $id: $type
    };
    (
        $id:ident, $($t1:ident),* ; $type:ty, $($t2:ty),*
    ) => {
        $id: $type, arg_list!($($t1),* ; $($t2),*)
    }
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
        $(#[$service_attr])*
        pub mod $svc_name {
            use super::*;

            use $crate::tokio::sync::mpsc::{self, Sender, Receiver};
            use $crate::serde_json::{self, Value};
            use $crate::serde::{Serialize, Deserialize};
            use $crate::anyhow::{Result, anyhow};
            use $crate::async_trait;

            mod method {
                $(
                    pub mod $method_name {
                        #[derive(super::super::Deserialize, super::super::Serialize)]
                        pub struct Argument {
                            $(pub $arg_id : $arg_ty),*
                        }

                        #[derive(super::super::Deserialize, super::super::Serialize)]
                        pub struct Payload {
                            pub method: &'static str,
                            pub arg: Argument,
                        }
                    }
                )*
            }

            #[async_trait]
            pub trait Service: Clone + Send + 'static {
                $(
                    $(#[$method_attr])*
                    async fn $method_name(&mut self, $($arg_id : $arg_ty),* ) -> $output;
                )*
            }

            #[derive(Clone)]
            pub struct Client {
                tx: Sender<(Sender<String>, String)>,
            }

            impl Client {

                $(
                    pub async fn $method_name(&self, $($arg_id : $arg_ty),* ) -> Result<$output> {
                    let args = method::$method_name::Payload {
                        method: stringify!($method_name),
                        arg: method::$method_name::Argument {
                            $($arg_id),*
                        }
                    };
                    let s = self.call(serde_json::to_string(&args)?).await?;
                    Ok($crate::serde_json::from_str(&s)?)
                })*

                pub fn with_server(tx: Sender<(Sender<String>, String)>) -> Self {
                    Self {
                        tx
                    }
                }

                async fn call(&self, s: String) -> Result<String> {
                    let (tx, mut rx) = mpsc::channel(100);
                    let server_tx = self.tx.clone();
                    server_tx.send((tx, s)).await?;
                    if let Some(s) = rx.recv().await {
                        Ok(s)
                    } else {
                        Err(anyhow!("receive failed"))
                    }
                }
            }


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
                        match self.handle1().await {
                            Ok(()) => {

                            }
                            Err(e) => {

                            }
                        }
                    }
                }
                async fn handle1(&mut self) -> Result<()> {

                    match self.rx.recv().await {
                        Some((tx, s)) => {
                            let v: Value = $crate::serde_json::from_str(&s)?;
                            if let Some(m) = v.get("method").map(|v| v.as_str()) {
                                if let Some(m) = m {
                                    match m {
                                    $(
                                        stringify!($method_name) => {
                                            if let Some(av) = v.get("arg") {
                                                let arg: method::$method_name::Argument = serde_json::from_value(av.clone())?;
                                                let ret = self.svc.$method_name (
                                                    $(arg.$arg_id),*
                                                ).await;
                                                let ret = serde_json::to_string(&ret)?;
                                                tx.send(ret).await?;
                                                Ok(())
                                            } else {
                                                Err(anyhow!("expected arg field"))
                                            }

                                        }
                                    )*
                                    _ => {
                                        Err(anyhow!("unexpected method name"))
                                    }
                                }
                                } else {
                                    Err(anyhow!("expected string"))
                                }
                            } else {
                                Err(anyhow!("expected method name"))
                            }
                        }
                        None => {Err(anyhow!("expected sender"))}
                    }
                }
            }
        }
    };
}
