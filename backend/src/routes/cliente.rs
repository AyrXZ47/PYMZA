use axum::{
    extract::State,
    Json,
};
use chrono::{Datelike, Utc};

use crate::auth::{es_correo_valido, EmpresaSession};
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
            "message": "CURP inválida: debe tener estructura CURP válida (18 caracteres, mayúsculas, fecha coherente, sexo, entidad y dígito verificador correctos)"
        }));
    }

    if let Some(correo) = &payload.correo {
        if !es_correo_valido(correo) {
            return Json(serde_json::json!({
                "status": "error",
                "message": "Correo inválido"
            }));
        }
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
        correo: payload.correo,
        // La verificación fuerte es por OTP (verificaciones); siempre nace false.
        telefono_verificado: false,
        // La INE se verifica después (KYC por OCR); siempre nace false.
        ine_verificada: false,
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

/// Valor de un carácter para el dígito verificador, según la tabla del
/// Instructivo Normativo RENAPO (DOF 18-10-2021): los dígitos se conservan y
/// las letras toman su valor base36 con el salto por Ñ (=24). A=10…N=23,
/// O=25…Z=36. Solo el residuo mod 10 afecta el cálculo.
fn valor_curp(c: u8) -> u32 {
    match c {
        b'0'..=b'9' => (c - b'0') as u32,
        b'A'..=b'N' => (c - b'A') as u32 + 10,
        b'O'..=b'Z' => (c - b'O') as u32 + 25,
        _ => 0, // inalcanzable: es_curp_valida ya filtró a A-Z/0-9 (una CURP real no contiene Ñ)
    }
}

/// Dígito verificador (18º carácter) de una CURP: suma ponderada de los 17
/// primeros caracteres — posición i (1-indexada) × peso 19-i, es decir pesos
/// 18..2 — módulo 10; dígito = (10 - suma mod 10) mod 10. Equivalente a
/// exigir que la suma ponderada de los 18 caracteres (el 18º con peso 1)
/// sea ≡ 0 mod 10.
/// Fuente: Instructivo Normativo para la Asignación de la CURP
/// (RENAPO/SEGOB, DOF 18-10-2021); reproducción con ejemplo en
/// https://curp.readthedocs.io/es/latest/instructivo/verificacion.html
fn digito_verificador(curp17: &str) -> char {
    let suma: u32 = curp17
        .as_bytes()
        .iter()
        .enumerate()
        .map(|(i, &c)| valor_curp(c) * (18 - i as u32))
        .sum();
    let d = (10 - suma % 10) % 10;
    (b'0' + d as u8) as char
}

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
    let anio2 = (b[4] - b'0') as i32 * 10 + (b[5] - b'0') as i32;
    let mes = (b[6] - b'0') as i32 * 10 + (b[7] - b'0') as i32;
    let dia = (b[8] - b'0') as i32 * 10 + (b[9] - b'0') as i32;
    if !(1..=12).contains(&mes) {
        return false;
    }
    // Coherencia real de fecha: días según el mes, incluidos bisiestos.
    // YY ≤ los dos dígitos del año actual → siglo XXI; si no, siglo XX.
    let anio = if anio2 <= Utc::now().year() % 100 { 2000 + anio2 } else { 1900 + anio2 };
    let bisiesto = (anio % 4 == 0 && anio % 100 != 0) || anio % 400 == 0;
    let dias_mes = match mes {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ => if bisiesto { 29 } else { 28 },
    };
    if !(1..=dias_mes).contains(&dia) {
        return false;
    }
    // Posición 11: sexo
    if b[10] != b'H' && b[10] != b'M' {
        return false;
    }
    // Posiciones 12-13: entidad federativa de nacimiento
    let entidad = std::str::from_utf8(&b[11..13]).expect("CURP ASCII");
    if !ENTIDADES_CURP.contains(&entidad) {
        return false;
    }
    // Posición 18: dígito verificador oficial. Una letra ahí no puede
    // coincidir con el dígito calculado, así que queda rechazada.
    digito_verificador(&curp[..17]) == b[17] as char
}

#[cfg(test)]
mod tests {
    use super::*;

    // CURPs del seed (scripts/seed.js). RAMJ y GARV traen el dígito
    // verificador corregido según el Instructivo RENAPO DOF 18-10-2021
    // (los originales 03/09 no pasan el algoritmo oficial); GAML ya era válida.
    const CURPS_SEED: [&str; 3] = [
        "RAMJ920215MDFMZR05",
        "GAML930528HDFLNR05",
        "GARV850710MCHLRN02",
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
        for curp in CURPS_SEED {
            assert!(es_curp_valida(curp), "{} debería ser válida", curp);
        }
    }

    #[test]
    fn digito_verificador_sigue_el_instructivo_oficial() {
        // Ejemplo worked-out del Instructivo (DOF 18-10-2021), reproducido en
        // https://curp.readthedocs.io/es/latest/instructivo/verificacion.html
        assert!(es_curp_valida("SABC560626MDFLRN01"));
        assert_eq!(digito_verificador("SABC560626MDFLRN0"), '1');
    }

    #[test]
    fn rechaza_formato_valido_con_digito_verificador_malo() {
        // Las CURP originales del seed: formato correcto, dígito incorrecto.
        assert!(!es_curp_valida("RAMJ920215MDFMZR03")); // dv real: 5
        assert!(!es_curp_valida("GARV850710MCHLRN09")); // dv real: 2
        // Cualquier dígito distinto al calculado se rechaza.
        for curp in CURPS_SEED {
            let mutada = format!("{}{}", &curp[..17], ((curp.as_bytes()[17] - b'0' + 1) % 10) as char);
            assert!(
                !es_curp_valida(&mutada),
                "dígito mutado debe rechazar: {}",
                mutada
            );
        }
    }

    #[test]
    fn rechaza_minusculas() {
        assert!(!es_curp_valida("ramj920215mdfmzr05"));
    }

    #[test]
    fn rechaza_longitud_incorrecta() {
        assert!(!es_curp_valida(""));
        assert!(!es_curp_valida("RAMJ920215MDFMZR0"));
        assert!(!es_curp_valida("RAMJ920215MDFMZR050"));
    }

    #[test]
    fn rechaza_fecha_invalida() {
        assert!(!es_curp_valida("RAMJ921315MDFMZR03")); // mes 13
        assert!(!es_curp_valida("RAMJ920232MDFMZR03")); // día 32
        assert!(!es_curp_valida("RAMJ92M215MDFMZR03")); // año con letra
    }

    #[test]
    fn rechaza_fecha_incoherente_con_calendario() {
        // dígito verificador correcto en todos los casos: solo la fecha falla
        assert!(!es_curp_valida("RAZJ920230MDFMMN07")); // 30 de febrero
        assert!(!es_curp_valida("RAZJ920431MDFMMN06")); // 31 de abril
        assert!(!es_curp_valida("RAZJ930229MDFMMN03")); // 29-feb-1993, no bisiesto
    }

    #[test]
    fn acepta_29_de_febrero_solo_en_bisiestos() {
        assert!(es_curp_valida("RAZJ920229MDFMMN06")); // 29-feb-1992, bisiesto
        assert!(es_curp_valida("RAZJ040229MDFMMN06")); // 29-feb-2004, bisiesto
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
        let json = r#"{"curp":"RAMJ920215MDFMZR05","nombre_completo":"Janeth Ramos Zamora","score":720,"nivel_riesgo":"Bajo","historial_pagos":"Puntual en 2 empresas de la red","direccion":"Av. Juárez 123, Centro","telefono":"5551234567"}"#;
        let cliente: Cliente = serde_json::from_str(json).expect("seed sin alerta debe deserializar");
        assert!(cliente.alerta.is_none());
    }

    #[test]
    fn cliente_sin_alerta_no_serializa_alerta() {
        let json = r#"{"curp":"RAMJ920215MDFMZR05","nombre_completo":"Janeth Ramos Zamora","score":720,"nivel_riesgo":"Bajo","historial_pagos":"Puntual en 2 empresas de la red","direccion":"Av. Juárez 123, Centro","telefono":"5551234567"}"#;
        let cliente: Cliente = serde_json::from_str(json).unwrap();
        assert!(!serde_json::to_string(&cliente).unwrap().contains("alerta"));
    }

    #[test]
    fn cliente_con_alerta_serializa_y_roundtripea() {
        let mut cliente: Cliente = serde_json::from_str(
            r#"{"curp":"RAMJ920215MDFMZR05","nombre_completo":"Janeth Ramos Zamora","score":720,"nivel_riesgo":"Bajo","historial_pagos":"Puntual en 2 empresas de la red","direccion":"Av. Juárez 123, Centro","telefono":"5551234567"}"#,
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

    #[test]
    fn documento_previo_ola_3_usa_defaults_nuevos() {
        // Docs antiguos sin `correo` ni `telefono_verificado` se leen con
        // defaults y `telefono_verificado` serializa siempre (lo lee la UI).
        let json = r#"{"curp":"GAML930528HDFLNR05","nombre_completo":"Gabriel Martínez López","score":640,"nivel_riesgo":"Medio","historial_pagos":"1 retraso de 5 días","direccion":"Calle 5 de Mayo 88","telefono":"5559876543"}"#;
        let cliente: Cliente = serde_json::from_str(json).unwrap();
        assert_eq!(cliente.correo, None);
        assert!(!cliente.telefono_verificado);
        let serializado = serde_json::to_string(&cliente).unwrap();
        assert!(serializado.contains(r#""telefono_verificado":false"#));
        assert!(!serializado.contains("correo"));
    }

    #[test]
    fn cliente_con_correo_roundtripea() {
        let json = r#"{"curp":"GAML930528HDFLNR05","nombre_completo":"Gabriel Martínez López","score":640,"nivel_riesgo":"Medio","historial_pagos":"1 retraso de 5 días","direccion":"Calle 5 de Mayo 88","telefono":"5559876543","correo":"gabriel@correo.mx","telefono_verificado":true}"#;
        let cliente: Cliente = serde_json::from_str(json).unwrap();
        assert_eq!(cliente.correo.as_deref(), Some("gabriel@correo.mx"));
        assert!(cliente.telefono_verificado);
        let de_vuelta: Cliente = serde_json::from_str(&serde_json::to_string(&cliente).unwrap()).unwrap();
        assert_eq!(de_vuelta.correo, cliente.correo);
        assert!(de_vuelta.telefono_verificado);
    }

    #[test]
    fn crear_cliente_req_sin_correo_usa_default() {
        let req: CrearClienteReq = serde_json::from_str(
            r#"{"curp":"GAML930528HDFLNR05","nombre_completo":"G","direccion":"X","telefono":"55"}"#,
        ).unwrap();
        assert_eq!(req.correo, None);
    }
}
