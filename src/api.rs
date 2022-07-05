use async_trait::async_trait;

#[async_trait]
pub trait API {
    async fn get_json(&self)-> Result<String, std::io::Error>;
}
