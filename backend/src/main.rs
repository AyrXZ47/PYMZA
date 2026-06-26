mod db;
mod models;
mod services;

use std::env;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <command>", args[0]);
        return;
    }

    match args[1].as_str() {
        "db" => db::connect().await,
        _ => println!("Unknown command"),
    }
}
