use axum::{
    extract::State,
    Json,
};

use crate::auth::EmpresaSession;
use crate::models::cliente::{AlertaMorosidad, Cliente, CrearClienteReq, ReportarReq};

pub async fn buscar_cliente(
    State(client): State<mongodb::Client>,
    _sesion: EmpresaSession,
    axum::extract::Path(curp): axum::extract::Path<String>
) -> Json<serde_json::Value> {
    let coll = client.database("pymza").collection::<Cliente>("clientes");

    match coll.find_one(
        mongodb::bson::doc! { "curp": curp },
        None
    ).await {
        Ok(Some(cliente)) => Json(serde_json::json!({
            "status": "success",
            "cliente": cliente
        })),
        Ok(None) => Json(serde_json::json!({
            "status": "not_found",
            "message": "Cliente no existe en la red PYMZA"
        })),
        Err(e) => {
            eprintln!("🚨 ERROR MONGODB: {:?}", e);
            Json(serde_json::json!({"status": "error"}))
        }
    }
}

pub async fn crear_cliente(
    State(client): State<mongodb::Client>,
    _sesion: EmpresaSession,
    Json(payload): Json<CrearClienteReq>,
) -> Json<serde_json::Value> {
    if !es_curp_valida(&payload.curp) {
        return Json(serde_json::json!({
            "status": "error",
            "message": "CURP inválida: debe tener estructura CURP válida (18 caracteres, mayúsculas, fecha, sexo y entidad correctos)"
        }));
    }

    let coll = client.database("pymza").collection::<Cliente>("clientes");

    if let Ok(Some(_)) = coll.find_one(mongodb::bson::doc! { "curp": &payload.curp }, None).await {
        return Json(serde_json::json!({
            "status": "error",
            "message": "Cliente ya existe en la red PYMZA"
        }));
    }

    let cliente = Cliente {
        curp: payload.curp,
        nombre_completo: payload.nombre_completo,
        score: 550,
        nivel_riesgo: "Medio".to_string(),
        historial_pagos: "Sin historial en la red".to_string(),
        direccion: payload.direccion,
        telefono: payload.telefono,
        alerta: None,
    };

    match coll.insert_one(&cliente, None).await {
        Ok(_) => Json(serde_json::json!({ "status": "success", "cliente": cliente })),
        Err(e) => {
            eprintln!("🚨 ERROR MONGODB: {:?}", e);
            Json(serde_json::json!({ "status": "error", "message": "Error al guardar el cliente" }))
        }
    }
}

pub async fn reportar_cliente(
    State(client): State<mongodb::Client>,
    sesion: EmpresaSession,
    axum::extract::Path(curp): axum::extract::Path<String>,
    Json(payload): Json<ReportarReq>,
) -> Json<serde_json::Value> {
    if payload.motivo.trim().is_empty() {
        return Json(serde_json::json!({
            "status": "error",
            "message": "Motivo es obligatorio"
        }));
    }

    let coll = client.database("pymza").collection::<Cliente>("clientes");
    let alerta = AlertaMorosidad {
        // ponytail: la alerta guarda el correo (tenant key), no el nombre
        // comercial; si la UI necesita el nombre tendrá que enriquecer en lectura.
        empresa: sesion.correo,
        motivo: payload.motivo,
    };
    // ponytail: un cliente con reporte previo se sobrescribe con el último;
    // el techo es un único flag por cliente. Multi-reportes (historial de
    // alertas) requerirían un array `alertas` en el documento.
    let update = mongodb::bson::doc! {
        "$set": { "alerta": { "empresa": &alerta.empresa, "motivo": &alerta.motivo } }
    };

    match coll.update_one(mongodb::bson::doc! { "curp": &curp }, update, None).await {
        Ok(res) if res.matched_count == 1 => Json(serde_json::json!({
            "status": "success",
            "alerta": alerta
        })),
        Ok(_) => Json(serde_json::json!({
            "status": "not_found",
            "message": "Cliente no existe en la red PYMZA"
        })),
        Err(e) => {
            eprintln!("🚨 ERROR MONGODB: {:?}", e);
            Json(serde_json::json!({ "status": "error" }))
        }
    }
}

const ENTIDADES_CURP: [&str; 33] = [
    "AS", "BC", "BS", "CC", "CL", "CM", "CS", "CH", "DF", "DG",
    "GT", "GR", "HG", "JC", "MC", "MN", "MS", "NT", "NL", "OC",
    "PL", "QT", "QR", "SP", "SL", "SR", "TC", "TS", "TL", "VZ",
    "YN", "ZS", "NE",
];

fn es_curp_valida(curp: &str) -> bool {
    let b = curp.as_bytes();
    if b.len() != 18 {
        return false;
    }
    if !b.iter().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()) {
        return false;
    }
    // Posiciones 1-4: letras (iniciales de apellidos y nombre)
    if !b[..4].iter().all(|c| c.is_ascii_uppercase()) {
        return false;
    }
    // Posiciones 5-10: fecha de nacimiento YYMMDD
    if !b[4..10].iter().all(|c| c.is_ascii_digit()) {
        return false;
    }
    let mes = (b[6] - b'0') as i32 * 10 + (b[7] - b'0') as i32;
    let dia = (b[8] - b'0') as i32 * 10 + (b[9] - b'0') as i32;
    if !(1..=12).contains(&mes) || !(1..=31).contains(&dia) {
        return false;
    }
    // Posición 11: sexo
    if b[10] != b'H' && b[10] != b'M' {
        return false;
    }
    // Posiciones 12-13: entidad federativa de nacimiento
    let entidad = std::str::from_utf8(&b[11..13]).expect("CURP ASCII");
    ENTIDADES_CURP.contains(&entidad)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CURPS_SEED: [&str; 3] = [
        "RAMJ920215MDFMZR03",
        "GAML930528HDFLNR05",
        "GARV850710MCHLRN09",
    ];

    #[test]
    fn es_curp_valida_acepta_las_curps_del_seed() {
        for curp in CURPS_SEED {
            assert!(es_curp_valida(curp), "CURP del seed debe ser válida: {}", curp);
        }
    }

    #[test]
    fn es_curp_valida_rechaza_curps_invalidas() {
        assert!(!es_curp_valida(""));
        assert!(!es_curp_valida("RAMJ920215MDFMZR0")); // 17 chars
        assert!(!es_curp_valida("RAMJ920215MDFMZR031")); // 19 chars
        assert!(!es_curp_valida("RAMJ920215MDFMZR0!")); // char no alfanumérico
    }

    #[test]
    fn curps_del_seed_son_validas() {
        for curp in ["RAMJ920215MDFMZR03", "GAML930528HDFLNR05", "GARV850710MCHLRN09"] {
            assert!(es_curp_valida(curp), "{} debería ser válida", curp);
        }
    }

    #[test]
    fn rechaza_minusculas() {
        assert!(!es_curp_valida("ramj920215mdfmzr03"));
    }

    #[test]
    fn rechaza_longitud_incorrecta() {
        assert!(!es_curp_valida(""));
        assert!(!es_curp_valida("RAMJ920215MDFMZR0"));
        assert!(!es_curp_valida("RAMJ920215MDFMZR030"));
    }

    #[test]
    fn rechaza_fecha_invalida() {
        assert!(!es_curp_valida("RAMJ921315MDFMZR03")); // mes 13
        assert!(!es_curp_valida("RAMJ920232MDFMZR03")); // día 32
        assert!(!es_curp_valida("RAMJ92M215MDFMZR03")); // año con letra
    }

    #[test]
    fn rechaza_sexo_invalido() {
        assert!(!es_curp_valida("RAMJ920215XDFMZR03"));
    }

    #[test]
    fn rechaza_entidad_invalida() {
        assert!(!es_curp_valida("RAMJ920215MXXMZR03"));
    }

    #[test]
    fn deserializa_documento_seed_sin_alerta() {
        let json = r#"{"curp":"RAMJ920215MDFMZR03","nombre_completo":"Janeth Ramos Zamora","score":720,"nivel_riesgo":"Bajo","historial_pagos":"Puntual en 2 empresas de la red","direccion":"Av. Juárez 123, Centro","telefono":"5551234567"}"#;
        let cliente: Cliente = serde_json::from_str(json).expect("seed sin alerta debe deserializar");
        assert!(cliente.alerta.is_none());
    }

    #[test]
    fn cliente_sin_alerta_responde_igual_que_hoy() {
        let json = r#"{"curp":"RAMJ920215MDFMZR03","nombre_completo":"Janeth Ramos Zamora","score":720,"nivel_riesgo":"Bajo","historial_pagos":"Puntual en 2 empresas de la red","direccion":"Av. Juárez 123, Centro","telefono":"5551234567"}"#;
        let cliente: Cliente = serde_json::from_str(json).unwrap();
        assert!(!serde_json::to_string(&cliente).unwrap().contains("alerta"));
    }

    #[test]
    fn cliente_con_alerta_serializa_y_roundtripea() {
        let mut cliente: Cliente = serde_json::from_str(
            r#"{"curp":"RAMJ920215MDFMZR03","nombre_completo":"Janeth Ramos Zamora","score":720,"nivel_riesgo":"Bajo","historial_pagos":"Puntual en 2 empresas de la red","direccion":"Av. Juárez 123, Centro","telefono":"5551234567"}"#,
        ).unwrap();
        cliente.alerta = Some(AlertaMorosidad {
            empresa: "Ferretería El Tornillo".to_string(),
            motivo: "Desapareció con deuda pendiente".to_string(),
        });

        let json = serde_json::to_string(&cliente).unwrap();
        let de_vuelta: Cliente = serde_json::from_str(&json).unwrap();
        let alerta = de_vuelta.alerta.expect("alerta debe sobrevivir al roundtrip");
        assert_eq!(alerta.empresa, "Ferretería El Tornillo");
        assert_eq!(alerta.motivo, "Desapareció con deuda pendiente");
    }
}
