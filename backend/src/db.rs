use mongodb::{Client, ClientOptions};
use std::env;

pub async fn connect() -> Result<Client, Box<dyn std::error::Error>> {
    let uri = env::var("MONGODB_URI").unwrap_or_else(|_| "mongodb://localhost:27017/pymza".to_string());
    let client_options = ClientOptions::parse(&uri).await?;
    let client = Client::with_options(client_options)?;
    println!("Connected to MongoDB!");
    Ok(client)
}
