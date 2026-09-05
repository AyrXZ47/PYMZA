//! OCR real (ola 5): el motor es el **binario `tesseract`** invocado como
//! proceso hijo con timeout — cero crates de OCR (decision log ola 5).
//! La imagen se escribe a un archivo temporal y se borra al terminar (la
//! imagen NUNCA se persiste — privacy by design). La extracción de datos
//! sobre el texto la hacen las funciones puras `buscar_curp`/`buscar_monto`,
//! testeadas sin DB ni binario.

use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};

/// Límite del contrato: tesseract no puede colgar el request para siempre.
const TIMEOUT_SECS: u64 = 30;

#[derive(Debug)]
pub enum OcrError {
    /// El binario `tesseract` no está en PATH, falló o venció el timeout.
    /// El handler lo mapea a 500 con mensaje claro (sin panic).
    NoDisponible,
}

/// Idioma de tesseract: env `OCR_LANG`, default "spa". Un placeholder vacío
/// (copiado de .env.example) NO activa un idioma raro — mismo patrón que las
/// env WHATSAPP_* de otp.rs.
fn idioma_ocr() -> String {
    std::env::var("OCR_LANG")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "spa".to_string())
}

/// Extensión del archivo temporal según el mime (los handlers solo aceptan
/// estos tres; cualquier otro ya se rechazó como 400).
fn extension(mime: &str) -> &'static str {
    match mime {
        "image/jpeg" => "jpg",
        "image/webp" => "webp",
        _ => "png",
    }
}

/// Ruta única del temporal (pid + contador atómico): suficiente sin una
/// crate de tempfiles. TESSDATA_PREFIX se hereda del entorno: tesseract lo
/// usa nativamente, aquí no se implementa nada.
fn ruta_temporal(ext: &str) -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("pymza_ocr_{}_{}.{}", std::process::id(), n, ext))
}

/// Corre tesseract sobre la imagen: escribe un temporal, invoca
/// `tesseract <tmp> stdout -l <OCR_LANG> --psm 6` con timeout y borra el
/// temporal SIEMPRE (éxito, error o timeout). `kill_on_drop` mata al hijo si
/// el timeout dispara mientras espera. Cualquier fallo del binario (ausente
/// en PATH, exit != 0, timeout) → `NoDisponible`, sin panic.
pub async fn extraer_texto(bytes: &[u8], mime: &str) -> Result<String, OcrError> {
    let ruta = ruta_temporal(extension(mime));
    std::fs::write(&ruta, bytes).map_err(|_| OcrError::NoDisponible)?;

    let hijo = tokio::process::Command::new("tesseract")
        .arg(&ruta)
        .arg("stdout")
        .arg("-l")
        .arg(idioma_ocr())
        .arg("--psm")
        .arg("6")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn();

    let salida = match hijo {
        Ok(child) => {
            let espera = child.wait_with_output();
            tokio::time::timeout(std::time::Duration::from_secs(TIMEOUT_SECS), espera).await
        }
        Err(e) => {
            // tesseract ausente en PATH (o sin permiso de ejecución).
            let _ = std::fs::remove_file(&ruta);
            eprintln!("🚨 tesseract no se pudo ejecutar: {e}");
            return Err(OcrError::NoDisponible);
        }
    };

    // El temporal se borra en todos los caminos: la imagen no se persiste.
    let _ = std::fs::remove_file(&ruta);

    match salida {
        Ok(Ok(out)) if out.status.success() => {
            Ok(String::from_utf8_lossy(&out.stdout).into_owned())
        }
        Ok(Ok(out)) => {
            eprintln!("🚨 tesseract falló: {}", String::from_utf8_lossy(&out.stderr));
            Err(OcrError::NoDisponible)
        }
        Ok(Err(e)) => {
            eprintln!("🚨 tesseract no se pudo ejecutar: {e}");
            Err(OcrError::NoDisponible)
        }
        Err(_) => Err(OcrError::NoDisponible), // timeout de 30s
    }
}

/// Busca la primera CURP en el texto de OCR. Tolerante al ruido: ignora TODO
/// carácter no alfanumérico entre caracteres (espacios, guiones, ligaduras,
/// saltos de línea) y sube minúsculas. Solo la FORMA (no el dígito
/// verificador): la coincidencia exacta contra el cliente la hace el handler.
/// Función PURA.
pub fn buscar_curp(texto: &str) -> Option<String> {
    let limpio: Vec<u8> = texto
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_uppercase() as u8)
        .collect();
    limpio
        .windows(18)
        .find(|w| forma_curp(w))
        .map(|w| String::from_utf8(w.to_vec()).expect("solo ASCII alfanumérico"))
}

/// Estructura CURP de 18: 4 letras, fecha YYMMDD, sexo H/M, entidad (2
/// letras), 3 letras de apellidos, homoclave (letra o dígito) y dígito
/// verificador. La homoclave acepta ambos porque los CURP antiguos la llevan
/// numérica (todas las del seed: …R05 → '0' en la posición 17).
fn forma_curp(w: &[u8]) -> bool {
    w[..4].iter().all(|c| c.is_ascii_uppercase())
        && w[4..10].iter().all(|c| c.is_ascii_digit())
        && (w[10] == b'H' || w[10] == b'M')
        && w[11..16].iter().all(|c| c.is_ascii_uppercase())
        && (w[16].is_ascii_uppercase() || w[16].is_ascii_digit())
        && w[17].is_ascii_digit()
}

/// Busca el monto del recibo en el texto de OCR: "$1,234.56", "1234.56
/// MXN", "TOTAL: $450.00"… Función PURA. Preferencia: el primer monto con
/// `$` o con sufijo MXN/PESOS; si no hay ninguno, el primer número decimal
/// suelto (fechas y folios enteros no cuentan — evita falsos positivos).
pub fn buscar_monto(texto: &str) -> Option<f64> {
    let t: Vec<char> = texto.chars().collect();
    let mut suelto: Option<f64> = None;
    let mut i = 0;
    while i < t.len() {
        if t[i] != '$' && !t[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        // '$' con o sin espacios antes de los dígitos.
        let con_moneda = t[i] == '$';
        let mut j = i;
        if con_moneda {
            j += 1;
            while j < t.len() && t[j].is_whitespace() {
                j += 1;
            }
        }
        if !t.get(j).map_or(false, |c| c.is_ascii_digit()) {
            i += 1;
            continue;
        }
        let mut fin = j;
        while fin < t.len() && (t[fin].is_ascii_digit() || t[fin] == ',' || t[fin] == '.') {
            fin += 1;
        }
        let token: String = t[j..fin].iter().collect();
        if let Some(monto) = parsear_monto(&token) {
            if con_moneda {
                return Some(monto);
            }
            let sufijo: String = t[fin..]
                .iter()
                .skip_while(|c| c.is_whitespace())
                .take(5)
                .collect::<String>()
                .to_ascii_uppercase();
            if sufijo.starts_with("MXN") || sufijo.starts_with("PESOS") {
                return Some(monto);
            }
            if suelto.is_none() && token.contains('.') {
                suelto = Some(monto);
            }
        }
        i = fin.max(i + 1);
    }
    suelto
}

/// Convierte un token numérico ("1,234.56") a f64 o lo rechaza: a lo sumo un
/// punto, decimales de ≤2 dígitos y comas solo como grupos de miles de 3.
/// Función PURA, testeada.
fn parsear_monto(token: &str) -> Option<f64> {
    let (entera, decimal) = match token.split_once('.') {
        Some((e, d)) => (e, d),
        None => (token, ""),
    };
    if !decimal.is_empty()
        && (decimal.len() > 2 || !decimal.bytes().all(|c| c.is_ascii_digit()))
    {
        return None;
    }
    let grupos: Vec<&str> = entera.split(',').collect();
    let comas_validas = grupos.len() == 1
        || (!grupos[0].is_empty()
            && grupos[0].len() <= 3
            && grupos[1..].iter().all(|g| g.len() == 3));
    if !comas_validas {
        return None;
    }
    let digitos: String = entera.chars().filter(|c| c.is_ascii_digit()).collect();
    if digitos.is_empty() {
        return None;
    }
    let mut valor: f64 = digitos.parse().ok()?;
    if !decimal.is_empty() {
        valor += format!("0.{decimal}").parse::<f64>().ok()?;
    }
    Some(valor)
}

/// Nombre en la INE: la línea anterior a la de la CURP, si parece un nombre
/// (≥2 palabras, solo letras y espacios). Heurística v1 para el fixture y la
/// UI; techo: validar contra RENAPO/proveedor KYC (ola 7). Función PURA.
pub fn buscar_nombre(texto: &str) -> Option<String> {
    let lineas: Vec<&str> = texto.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    let i = lineas.iter().position(|l| buscar_curp(l).is_some())?;
    let nombre = lineas.get(i.checked_sub(1)?)?;
    let parece_nombre = nombre.split_whitespace().count() >= 2
        && nombre.chars().all(|c| c.is_alphabetic() || c.is_whitespace());
    if parece_nombre {
        Some((*nombre).to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CURP_SEED: &str = "GAML930528HDFLNR05";

    #[test]
    fn buscar_curp_encuentra_texto_limpio() {
        assert_eq!(buscar_curp(CURP_SEED), Some(CURP_SEED.to_string()));
        assert_eq!(
            buscar_curp("MARIA GOMEZ LOPEZ\nGAML930528HDFLNR05\nDOMICILIO"),
            Some(CURP_SEED.to_string())
        );
    }

    #[test]
    fn buscar_curp_tolera_ruido_de_ocr() {
        // espacios y ligaduras entre caracteres
        assert_eq!(
            buscar_curp("G A M L 9305 28 H D F L N R 05"),
            Some(CURP_SEED.to_string())
        );
        // salto de línea dentro de la CURP
        assert_eq!(
            buscar_curp("MARIA GOMEZ LOPEZ\nGAML930528\nHDFLNR05"),
            Some(CURP_SEED.to_string())
        );
        // minúsculas (OCR de baja calidad)
        assert_eq!(
            buscar_curp("gaml930528hdflnr05"),
            Some(CURP_SEED.to_string())
        );
        // texto con líneas curadas alrededor y guiones
        assert_eq!(
            buscar_curp("CLAVE DE ELECTOR\nGAML-930528-HDFLNR-05\n01/2026"),
            Some(CURP_SEED.to_string())
        );
    }

    #[test]
    fn buscar_curp_ignora_texto_sin_curp() {
        assert_eq!(buscar_curp("CREDENCIAL PARA VOTAR"), None);
        assert_eq!(buscar_curp(""), None);
        // folios y teléfonos no calzan la forma (letras al inicio, H/M, etc.)
        assert_eq!(buscar_curp("FOLIO 0123456789ABCDEF12"), None);
        // forma incompleta: 17 caracteres válidos
        assert_eq!(buscar_curp("GAML930528HDFLNR5"), None);
    }

    #[test]
    fn buscar_curp_acepta_homoclave_numerica_como_las_del_seed() {
        // La homoclave (posición 17) en los CURP del seed es '0' (dígito).
        assert_eq!(buscar_curp("RAMJ920215MDFMZR05"), Some("RAMJ920215MDFMZR05".to_string()));
        // y también con homoclave de letra (CURP moderna)
        assert_eq!(buscar_curp("SABC560626MDFLRA01"), Some("SABC560626MDFLRA01".to_string()));
    }

    #[test]
    fn los_parsers_leen_el_output_real_del_fixture() {
        // Salida verificada de:
        //   tesseract backend/scripts/fixture_ine.png stdout -l spa --psm 6
        // La CURP va agrupada (como en una credencial real) para que tesseract
        // no confunda el '0' con 'O'; buscar_curp la reconstruye.
        let texto = "MARIA GOMEZ LOPEZ\nGAML 930528 HDFLNR 05\n";
        assert_eq!(buscar_curp(texto), Some(CURP_SEED.to_string()));
        assert_eq!(buscar_nombre(texto).as_deref(), Some("MARIA GOMEZ LOPEZ"));
    }

    #[test]
    fn buscar_monto_lee_formatos_del_contrato() {
        assert_eq!(buscar_monto("IMPORTE: $1,234.56"), Some(1234.56));
        assert_eq!(buscar_monto("TOTAL: $450.00"), Some(450.0));
        assert_eq!(buscar_monto("1234.56 MXN"), Some(1234.56));
        assert_eq!(buscar_monto("890.50 PESOS"), Some(890.5));
        assert_eq!(buscar_monto("$ 2,000.0"), Some(2000.0));
    }

    #[test]
    fn buscar_monto_lee_primer_con_signo_de_peso() {
        assert_eq!(buscar_monto("CUOTA $300.00 RESTANTE $1,500.00"), Some(300.0));
    }

    #[test]
    fn buscar_monto_no_confunde_fechas_ni_folios() {
        // fecha y folio enteros: sin punto decimal ni sufijo → None
        assert_eq!(buscar_monto("FECHA 2026-07-22 FOLIO 12345"), None);
        assert_eq!(buscar_monto("CREDENCIAL PARA VOTAR"), None);
        assert_eq!(buscar_monto(""), None);
    }

    #[test]
    fn buscar_monto_con_ruido_de_ocr() {
        // TOTAL mal leído no rompe el monto pegado al '$'
        assert_eq!(buscar_monto("T0TAL: $1,234.56"), Some(1234.56));
        // número con ruido no numérico pegado: se para en el primer carácter ajeno
        assert_eq!(buscar_monto("TOTAL $329.9 |"), Some(329.9));
    }

    #[test]
    fn parsear_monto_rechaza_tokens_malformados() {
        assert_eq!(parsear_monto(""), None);
        assert_eq!(parsear_monto("12,34"), None); // comas que no son miles
        assert_eq!(parsear_monto("1.2.3"), None); // dos puntos decimales
        assert_eq!(parsear_monto("1234.567"), None); // 3 decimales (ruido)
        assert_eq!(parsear_monto("1,234,567"), Some(1234567.0)); // miles ok
    }
}
