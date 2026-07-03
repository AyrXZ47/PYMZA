use mongodb::{Client, options::ClientOptions, Database};
use std::env;

pub async fn connect() -> Result<Database, Box<dyn std::error::Error>> {
    let uri = env::var("MONGODB_URI").unwrap_or_else(|_| "mongodb://localhost:27017".to_string());
    let client_options = ClientOptions::parse(&uri).await?;
    let client = Client::with_options(client_options)?;
    println!("Connected to MongoDB!");
    Ok(client.database("pymza"))
}
