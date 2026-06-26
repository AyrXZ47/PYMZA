use mongodb::{Client, options::ClientOptions};
use std::env;

pub async fn connect() -> Result<(), Box<dyn std::error::Error>> {
    let uri = env::var("MONGODB_URI").expect("MONGODB_URI must be set");
    let client_options = ClientOptions::parse(&uri).await?;
    let client = Client::with_options(client_options)?;
    println!("Connected to MongoDB!");
    Ok(())
}
