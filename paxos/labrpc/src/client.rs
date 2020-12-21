use crate::anyhow::Result;
#[derive(Clone, Default)]
pub struct Client {}

impl Client {
    pub async fn call(&self, args: String) -> Result<String> {
        todo!();
    }
}
