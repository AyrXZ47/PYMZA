# Security Audit Report — PYMZA (ola 6, release gate)

- **Fecha:** 2026-09-06 · **Árbol:** `main` @ `99fd4b5` (integrado, ola 6)
- **Método:** `skills/security-audit`, 6 fases (recon → hunt paralelo → validación adversarial en vivo → reporte → findings.json validado → verificación independiente por finding con agentes frescos)
- **Resultado:** 8 confirmados (2 HIGH, 4 MEDIUM, 2 LOW) + 2 rechazados por evidencia + 6 hardening. **El despliegue NO puede salir con F1/F2 abiertos.**

## Alcance del hunt

- **Backend** (`backend/src/**`): inyección NoSQL (35 sinks revisados — limpio, queries con `doc!` de claves literales y valores tipados), tipos de body, panics alcanzables, reglas de dinero, OTP, tesseract/tempfile, rate limiter, CORS, límites de body.
- **Frontend** (`frontend/src/**`): construcción de URLs, sesión (localStorage), descarga de contrato (Content-Disposition → `download`), XSS (0 raw HTML — Dioxus escapa), fugas de token.
- **Focos pedidos por el release gate**: tenant isolation del PDF, bypass XFF del rate limiter, CORS, DoS con payloads base64 grandes.

## Hallazgos CONFIRMADOS

### F1 — HIGH — DoS total con 1 request: `plazo_meses` sin tope en evaluar → OOM (~86 GB)
`POST /api/creditos/evaluar` con `plazo_meses: i32` sin validar → `generar_plan_pagos` hace `(1..=plazo_meses).map(..).collect()` (credito.rs:33-49) → con `2147483647` reserva ~86 GB. **Confirmado en vivo**: RSS 20→30 GB en 12 s, camino al OOM (proceso terminado manualmente para proteger el entorno). Registro público → cualquier atacante mata el servicio con 1 request. Fix: validar `3..=12` y monto finito positivo en evaluar y autorizar.

### F2 — HIGH — `autorizar` persiste plazo sin validar: cartera/resumen congelados + contrato-PDF OOM persistente
`POST /api/creditos/autorizar` inserta el plan sin revalidar nada (credito.rs:488-504). Plan con `plazo_meses=2147483647`: **confirmado en vivo** — el plan quedó en `planes_pago` (el request cuelga después del insert, en el upsert del dashboard), `GET /api/creditos` cuelga (timeout 6 s) con el proceso al 66 % CPU, resumen/dashboard con stats stale permanentes, y `GET .../contrato` dispara el mismo OOM de F1 desde estado persistido (sin endpoint de borrado). Fix: misma validación de F1 en autorizar (+ script de limpieza si ya hay datos envenenados).

### F3 — MEDIUM — `telefono_verificado` falsificable (OTP nunca compara el teléfono del cliente)
`solicitar` crea el desafío con el teléfono del body **sin consultar clientes**; `confirmar` marca `telefono_verificado=true` filtrando **solo por curp** (verificacion.rs:114). **Confirmado en vivo**: cliente con teléfono real `5551234567` quedó verificado vía OTP enviado a `5550000001`. Cross-tenant (2 requests, sin contacto con el cliente). Secundarios: desafío no se consume en error (sin límite de intentos), SHA-256 sin salt de dominio 10^6. Fix: exigir/almacenar `clientes.telefono` en solicitar y compararlo en confirmar.

### F4 — MEDIUM (requires deployment testing) — rate limiter por peer-addr: bucket global tras el proxy de Railway
`GovernorConfigBuilder` sin `key_extractor` → bucket por IP del socket. **Sin bypass** (probado: XFF falsificado no restablece el bucket), PERO en Railway todas las conexiones comparten peer-addr → **un bucket global**: un atacante (o una ráfaga de usuarios legítimos) manda 21 requests y el login de TODOS da 429. La doc del propio crate (key_extractor.rs:55-58) lo advierte verbatim. La alternativa ingenua (`SmartIpKeyExtractor`) reintroduce spoofing de XFF. Fix: extractor custom que confíe en X-Forwarded-For solo para el peer del proxy de Railway; re-test en Railway.

### F5 — MEDIUM — carrera en `alta_empresa` sin índice único → tenant keys duplicadas (cross-tenant total)
Find-then-insert con ~100 ms de argon2 EN MEDIO (empresa.rs:25→32-41→50); no hay índice único en `empresas.correo` (db.rs solo crea el TTL de verificaciones). Dos registros simultáneos del mismo correo → ambos insertan → ambos JWT comparten `sub` → **cross-tenant total** para ese correo (planes, pagos, dashboard, recibos, alertas, contratos). Fix: índice único en db.rs + mapear el error de clave duplicada al 400 "Ya existe" (hoy el error de insert devolvería 200 "Error al registrar").

### F6 — MEDIUM — TOCTOU en `registrar_pago`: cuota duplicada y "Liquidado" falso
Check "Cuota ya registrada" sobre snapshot en memoria (credito.rs:583-589) + `insert_one` sin índice único `(plan_id, cuota)` (credito.rs:605). Doble-click basta: cuota doble, cobrado inflado (credito.rs:152,155), y `estado_plan` con `.len()` → **"Liquidado" con cuotas impagas** (credito.rs:67) — oculta la deuda de proximos_cobros/morosidad. Fix: índice único `(plan_id, cuota)` + mapear duplicado a 400.

### F7 — LOW — enumeración de correos de empresa (respuesta + timing)
"Ya existe una empresa registrada con ese correo" es oráculo directo (empresa.rs:25-30); el login corre argon2 (~100 ms) solo si el correo existe (auth.rs:134-160). El registro tiene además su timing oracle invertido (los NO registrados pagan el hash). Acotado por el rate limit público. Fix cuando el negocio lo pida: respuesta uniforme + argon2 dummy para correos desconocidos.

### F8 — LOW — carrera en el tope de 2 recibos (N2 de la ola 5, persiste)
count→insert sin índice (kyc.rs:189→202): recibos concurrentes superan el tope. **Precisión de fase 6**: el score SÍ decide `evaluar` (score>700 → capacidad 5000, credito.rs:448-449 — puede voltear Rechazado→Aprobado en la banda $2000-5000); lo que NO consulta score es `autorizar` (dinero persistido). Fix: mecanismo atómico de tope (contador condicional o transacción — el índice único por (curp,tipo) solo caparía 3).

## RECHAZADOS por evidencia dinámica (del hunt estático)

1. **`1e400` → inf → panic en `json!`**: refutado — el extractor JSON de axum rechaza con 400 "number out of range" antes del handler (probado, backend vivo). Los no-finitos no llegan por HTTP.
2. **`expect` de printpdf con emoji**: refutado — 'Crédito 🚀 émoji-test' → 200, PDF válido, el emoji se descarta del texto (probado, backend vivo).

## Hardening (sin daño demostrable — no son findings)

- H1 tempfile predecible en ocr.rs (race local de symlink; fix: `tempfile`/`O_EXCL`).
- H2 tesseract parsea imágenes attacker-controlled sin sandbox (timeout + kill_on_drop presentes).
- H3 db.rs sin connect_timeout / sin TimeoutLayer (resiliencia ante Mongo colgado).
- H4 campos de texto sin topes (motivo, nombres, producto — hasta ~3 MB por doc).
- H5 OTP cae a mock con código en logs si faltan WHATSAPP_* (documentado en API.md — revisar en Railway).
- H6 `sesion_ok` en 401 no borra el token de localStorage (higiénico, token ya muerto).
- H7 comparación de hash OTP no constante-tiempo (compara hashes SHA-256, no invertible).

## Lo que está BIEN (verificado)

- **Tenant isolation del PDF**: query con `_id` + `empresa` del token (credito.rs:686); **probado en vivo**: empresa B → 404 sobre plan de demo, cartera vacía. El check crítico del release gate PASA.
- **Inyección NoSQL**: 0 vectores (35 sinks con claves literales y valores tipados Rust).
- **JWT**: HS256 fijo (sin alg-confusion), exp obligatorio, `JWT_SECRET` con panic al arranque; argon2id sin fallback plaintext; `password` con `skip_serializing`.
- **CURP**: validación completa (RENAPO) en alta; los slices internos inalcanzables con input inválido.
- **KYC/recibos**: orden de validación correcto (mime → tamaño por largo b64 → base64 → 404).
- **XSS frontend**: 0 raw HTML; todos los datos del backend en text nodes escapados; token jamás en URL.
- **Descarga de contrato (frontend)**: filename JSON-escaped y asignado como propiedad DOM (no HTML); browser sanea el nombre; `HeaderValue::from_str` rechaza el header si el curp trae bytes raros (fallback `contrato.pdf`).
- **CORS**: lista explícita (no Any), default dev, sin bypass por sufijo de host (probado).
- **Rate limiter (local)**: sin bypass por XFF (probado); 429 con JSON del contrato.
- **1e400/emoji**: rechazados/descartados limpio (ver RECHAZADOS).

## Datos de las pruebas en vivo

Entorno: mongod local (mongo:latest del compose NO arranca en kernel 6.19 — SERVER-121912, hallazgo ambiental para V), backend `pymza_backend` compilado de `main` con DB local efímera y JWT efímera. Evidencia completa con outputs en `.workflow/audits/wave6.md`.
