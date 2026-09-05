use std::{env, time::Duration};

use mongodb::{
    bson::doc,
    options::{ClientOptions, IndexOptions},
    Client, IndexModel,
};

pub async fn connect() -> Result<Client, Box<dyn std::error::Error>> {
    // EL FIX ESTÁ AQUÍ: Usamos 127.0.0.1 directo para evitar el timeout de IPv6
    let uri = env::var("MONGODB_URI").unwrap_or_else(|_| "mongodb://127.0.0.1:27017".to_string());
    let mut client_options = ClientOptions::parse(&uri).await?;

    client_options.max_pool_size = Some(10);

    let client = Client::with_options(client_options)?;
    // O2 auditor ola 3: índice TTL sobre `verificaciones.expira_en`. Es
    // idempotente (create_index es no-op si ya existe) y no fatal: si falla
    // (p. ej. sin permisos en el cluster), el backend arranca igual y se
    // reintenta en el próximo arranque.
    if let Err(e) = crear_indice_ttl(&client).await {
        eprintln!("⚠️ No se pudo crear el índice TTL de verificaciones: {e}");
    }
    println!("--- Pool de conexiones MongoDB inicializado ---");
    Ok(client)
}

/// Índice TTL sobre `verificaciones.expira_en` (BSON date): Mongo borra los
/// desafíos vencidos (10 min). Con `expireAfterSeconds: 0` un documento
/// expira justo cuando el campo fecha queda en el pasado. NOTA: el TTL solo
/// aplica a campos BSON date — por eso `expira_en` se escribe como
/// `bson::DateTime` (no i64); docs viejos con i64 los limpia el flujo normal.
async fn crear_indice_ttl(client: &Client) -> Result<(), mongodb::error::Error> {
    let coll = client
        .database("pymza")
        .collection::<mongodb::bson::Document>("verificaciones");
    let index = IndexModel::builder()
        .keys(doc! { "expira_en": 1 })
        .options(IndexOptions::builder().expire_after(Duration::from_secs(0)).build())
        .build();
    coll.create_index(index, None).await.map(|_| ())
}
