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
    // F5 (auditoría ola 6): índice único en `empresas.correo` — cierra la
    // carrera find-then-insert de alta_empresa (dos inserts simultáneos del
    // mismo correo compartían la tenant key → cross-tenant). Idempotente; si
    // falla (dupes históricas o sin permisos), el backend arranca igual y se
    // reintenta en el próximo arranque.
    if let Err(e) = crear_indice_unico_empresa_correo(&client).await {
        eprintln!("⚠️ No se pudo crear el índice único de empresas.correo: {e}");
    }
    // Ola 7 (cierra F6): índice único PARCIAL en `pagos{plan_id, cuota}` con
    // `cuota > 0` — impide la carrera de doble pago de una cuota sin bloquear
    // los abonos (`cuota: 0`, que se acumulan legítimamente). Idempotente; si
    // falla, el backend arranca igual y se reintenta en el próximo arranque.
    if let Err(e) = crear_indice_unico_pago_cuota(&client).await {
        eprintln!("⚠️ No se pudo crear el índice único de pagos(plan_id, cuota): {e}");
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

/// Índice único sobre `empresas.correo`: el correo es la tenant key (claim
/// `sub` del JWT), así que dos empresas con el mismo correo compartirían TODO
/// (planes, pagos, dashboard, contratos). Con el índice, el segundo insert de
/// la carrera falla en Mongo mismo.
async fn crear_indice_unico_empresa_correo(client: &Client) -> Result<(), mongodb::error::Error> {
    let coll = client
        .database("pymza")
        .collection::<mongodb::bson::Document>("empresas");
    let index = IndexModel::builder()
        .keys(doc! { "correo": 1 })
        .options(IndexOptions::builder().unique(true).build())
        .build();
    coll.create_index(index, None).await.map(|_| ())
}

/// Índice único PARCIAL sobre `pagos{plan_id, cuota}` (ola 7): solo aplica a
/// `cuota > 0`, así una cuota no puede pagarse dos veces (carrera F6) mientras
/// los abonos (`cuota: 0`) pueden repetirse cuantas veces haga falta.
async fn crear_indice_unico_pago_cuota(client: &Client) -> Result<(), mongodb::error::Error> {
    let coll = client
        .database("pymza")
        .collection::<mongodb::bson::Document>("pagos");
    let index = IndexModel::builder()
        .keys(doc! { "plan_id": 1, "cuota": 1 })
        .options(
            IndexOptions::builder()
                .unique(true)
                .partial_filter_expression(doc! { "cuota": { "$gt": 0 } })
                .build(),
        )
        .build();
    coll.create_index(index, None).await.map(|_| ())
}
