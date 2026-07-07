use mongodb::{Client, options::ClientOptions};
use std::env;

pub async fn connect() -> Result<Client, Box<dyn std::error::Error>> {
    // EL FIX ESTÁ AQUÍ: Usamos 127.0.0.1 directo para evitar el timeout de IPv6
    let uri = env::var("MONGODB_URI").unwrap_or_else(|_| "mongodb://127.0.0.1:27017".to_string());
    let mut client_options = ClientOptions::parse(&uri).await?;
    
    client_options.max_pool_size = Some(10);
    
    let client = Client::with_options(client_options)?;
    println!("--- Pool de conexiones MongoDB inicializado ---");
    Ok(client)
}
