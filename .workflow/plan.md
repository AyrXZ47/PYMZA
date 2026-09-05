# Plan: PYMZA — Perfilación de Crédito y Cobranza para PYMES

> Single source of truth del trabajo. Commiteado, sobrevive cualquier sesión.
> SOLO la siguiente ola está detallada (plan rodante). Si una sesión muere, la
> nueva instancia reanuda desde este archivo — nunca desde memoria.

## Goal

PYMZA v1 en producción (Railway): una PYME real puede registrarse, iniciar
sesión con auth real (JWT), dar de alta y buscar clientes en la red, evaluar y
autorizar créditos con planes de pago aislados por empresa (multi-tenant), y
ver su dashboard — todo contra MongoDB Atlas. Medible: el flujo completo
registro → login → alta cliente → evaluar → autorizar → dashboard funciona en
la URL pública con cero datos cruzados entre empresas.

Lo que NO está en este plan (explícitamente fuera): la app de cobradores
("uber de cobranza") es un producto móvil separado; se evaluará cuando la red
de crédito esté viva. Tauri/escritorio: el producto es web primero.

## Stack & constraints

| Capa | Tech |
|---|---|
| Frontend | Dioxus 0.7.9 (pin `=0.7.9`) Rust → WASM + Tailwind v4. `frontend/AGENTS.md` es la referencia API obligatoria |
| Backend | Axum 0.6 / Tokio. Modularizado: `routes/`, `models/`, `auth.rs` (JWT HS256), `otp.rs` |
| DB | MongoDB Atlas (real) vía `MONGODB_URI` en `backend/.env` (gitignored). DB: `pymza`; colecciones: `empresas`, `clientes`, `planes_pago`, `dashboard_stats`, `verificaciones`, `pagos` (+ `recibos` desde ola 5) |
| Infra | Docker Compose existente; despliegue objetivo: Railway (ola 6; Dockerfile.backend instalará `tesseract-ocr` + `tesseract-ocr-spa`) |

Constraints:
- Secretos nunca al repo: `MONGODB_URI`, `JWT_SECRET`, credenciales de proveedores (WhatsApp, Stripe, CdC, KYC) solo en `.env` local.
- `Cargo.lock` gitignored (root `.gitignore`) — normal al añadir deps.
- NixOS: `dx` no compila Tailwind; el CSS compilado (`frontend/assets/tailwind.css`) está commiteado. Si una ola cambia clases Tailwind, regenerar con `frontend/tailwind.sh` y commitear el CSS.
- Sin CI. Verificación disponible: `cargo test` (backend y frontend nativo) + `cargo check --target wasm32-unknown-unknown` (frontend WASM).
- Backend necesita librerías dev de OpenSSL para compilar (driver mongodb con `openssl-tls`).
- OCR (ola 5): motor = binario `tesseract` invocado como proceso hijo (cero deps Rust); requiere `tesseract` + traineddata `spa` en el entorno (NixOS: `nix run nixpkgs#tesseract -- --version` para verificar; si falta, los endpoints devuelven error claro 500 "OCR no disponible").
- Release gate (ola 6): `skills/security-audit` con cero CRITICAL/HIGH antes de producción.
- Demo real en Atlas: `demo@pymza.mx` / `demo1234` (V re-seedeó 2026-09-04; docs actualizadas por planner).

## Waves

| Ola | Foco | Estado |
|-----|------|--------|
| 1 | Cimientos: JWT real + aislamiento multi-tenant + frontend partido en módulos | [x] auditada 2026-08-17 |
| 2 | Portal público: landing que venda, registro/login separados, tema claro/oscuro, `API_BASE` configurable | [x] auditada 2026-08-28 |
| 3 | Identidad verificable: CURP con dígito verificador, correo del cliente, OTP teléfono (WhatsApp/mock) | [x] auditada 2026-08-31 (APPROVED) |
| 4 | Cartera viva: registro de pagos + estados de plan + gráficas de impacto (SVG) + favicon | [x] auditada 2026-09-04 (APPROVED; N1-N3 informativas) |
| 5 | KYC/OCR real (subida de archivos, tesseract) + score alternativo por recibos de servicios | [x] auditada 2026-09-05 (APPROVED WITH EXCEPTIONS; E1 >2MB→413, E2 fixture no sirve para recibos — owner ola 6; humo UI pendiente de V) |
| 6 | Contrato PDF + Producción: Railway, CORS productivo, rate limiting, backups, security audit (release gate) | [ ] |
| 7 | Dinero (Stripe) + Ecosistema: roles, verificación CURP oficial (proveedor RENAPO), buró CdC (sandbox), open banking | [ ] |

> Re-segmentaciones documentadas en el decision log (2026-08-28, 2026-08-31).
> Estados: planificada → en vuelo → integrada → auditada → hecha.
> Actualizar después de cada paso, quien lo ejecute.

---

## Ola 5 (actual): KYC/OCR real + score alternativo por recibos

Contexto: la red de clientes ya verifica teléfono por OTP (ola 3) y la
cartera vive con pagos reales y gráficas (ola 4). Faltan las dos señales de
confianza que hacen al pitch de PYMZA: **la INE subida es real y coincide**
(KYC) y **el cliente sin buró obtiene score por sus recibos de servicios**
(motor de datos alternativos — idea original de PYMZA.md).

Decisiones de diseño (ponytail, razones en decision log):
- Motor OCR = **binario `tesseract`** invocado como proceso hijo con timeout
  (cero deps Rust para el motor; Docker lo instalará en la ola 6).
- Subida = **base64 en JSON** (no multipart): reqwest/wasm en el frontend es
  la única forma portable; archivos de INE/recibo son pequeños.
- **La imagen NO se guarda** (privacy by design): solo el resultado de la
  verificación. Techo: guardar si el negocio o regulación lo piden.
- Score por recibos = heurística v1: +25 por recibo legible (máx 2 recibos).
  El score alternativo real (historial de pagos de servicios) exige open
  banking — ola 7.

### Contrato API ola 5 (ambos executors implementan contra ESTO)

- **`Cliente`** gana `ine_verificada: bool` (default false, como los campos de
  la ola 3 — clientes previos los leen con default).
- **`POST /api/clientes/:curp/kyc`** (protegido): body
  `{archivo_b64: String, mime: String}`. Validaciones (trust boundary, en
  orden): cliente existe y pertenece a la red (404 si no); mime ∈
  {image/png, image/jpeg, image/webp} (400); tamaño decodificado ≤ 2 MB (400).
  Decodifica, corre tesseract (timeout 30s, idioma env `OCR_LANG` default
  "spa"), extrae la CURP del texto con la regex del formato CURP (función
  pura testada con texto ruidoso de OCR). Compara con el `curp` del path.
  Marca `ine_verificada = true` si coincide. Respuesta:
  `{status: "success", curp_ine: <string|null>, nombre_ine: <string|null>,
  coincide: bool, ine_verificada: bool}`. Si tesseract no está instalado o
  falla → 500 `{status:"error", message:"OCR no disponible en este
  servidor"}` sin panickear. Si no se encuentra CURP en el texto → success
  con `curp_ine: null, coincide: false` y mensaje.
- **`POST /api/clientes/:curp/recibos`** (protegegido): body
  `{archivo_b64, mime, tipo: "luz"|"agua"|"telefono"}` (tipo invalidado 400;
  mime/tamaño igual que kyc). OCR extrae el monto (regex `$`/pesos, función
  pura testada). Inserta en colección `recibos`:
  `{curp, empresa (correo del token), tipo, monto_leido (f64|null), fecha}`.
  Si el recibo es legible (monto encontrado o texto ≥50 chars), aplica bonus
  de score: `score += 25`, máximo 2 recibos por cliente (query a `recibos`;
  si ya hay 2 → 400 "Máximo 2 recibos por cliente"). Recalcula
  `nivel_riesgo` con función pura `nivel_por_score(score)`:
  `>=750 "Bajo"`, `>=550 "Medio"`, `<550 "Alto"` (tests). Actualiza el
  cliente. Respuesta:
  `{status: "success", monto_leido: <f64|null>, score, nivel_riesgo,
  recibos_contados: n}`.
- **`GET /api/clientes/:curp`** ya serializa los campos nuevos
  (`ine_verificada`) — el frontend solo dibuja.
- **`backend/src/ocr.rs`** (nuevo): `extraer_texto(bytes, mime) ->
  Result<String>` (escribe temp file, corre `tesseract <file> stdout -l
  $OCR_LANG --psm 6` con `tokio::process::Command` + timeout, borra temp),
  `buscar_curp(&str) -> Option<String>` (regex, función pura),
  `buscar_monto(&str) -> Option<f64>` (regex, función pura). El binario
  `tesseract` se busca en PATH; si no existe → error "OCR no disponible".
- **Deps nuevas backend: SOLO `base64 = "0.22"`.** Cero otras.
- **Fixture de prueba** (nuevo): `backend/scripts/fixture_ine.png` — imagen
  (PNG, ~600×400) con texto grande y legible que incluya una CURP válida
  (dígito verificador real, ej. una del seed) y un nombre. La genera el
  executor-1 en su worktree (la herramienta que prefiera) y se commitea, para
  que el humo de integración no dependa de una INE real.
- **`seed.js`**: actualizar el hash precomputado al de `demo1234` (la DB real
  de V ya usa demo1234; el hash commiteado está viejo — N1/O1 del auditor).
- **docs/API.md** + **.env.example** (`OCR_LANG=""`).
- **Frontend** (`alta_cliente.rs` o nuevo `kyc.rs` + `api.rs`):
  - Tras el alta o al buscar un cliente: panel "Verificar INE" — input file
    (accept image/*), lee bytes en wasm (js_sys/web_sys), codifica base64,
    llama kyc; muestra resultado (curp leída, coincide/no, badge "INE
    verificada" al lograrlo).
  - Panel "Score por recibos": select tipo (luz/agua/teléfono) + input file +
    botón → POST recibos → muestra score nuevo y nivel de riesgo.
  - Búsqueda por CURP: badges "✓ Teléfono" y "✓ INE" + score/nivel visibles.
  - `api.rs`: helpers `archivo_a_b64(bytes) -> String` (base64), `kyc_verificar`,
    `recibo_subir`. Tests en host: `archivo_a_b64`, parseo de respuestas,
    semáforo de estado del panel.
  - Deps nuevas frontend: SOLO `base64 = "0.22"`.
  - Regenerar CSS si hay clases nuevas (probable) y commitearlo.

### Mapa de propiedad de archivos

| Archivo/glob | Dueño |
|-----------|-------|
| `backend/Cargo.toml`, `backend/src/**`, `backend/scripts/**`, `docs/API.md`, `.env.example` | executor-1 |
| `frontend/src/**`, `frontend/tailwind.css`, `frontend/assets/**`, `frontend/Cargo.toml` | executor-2 |

Fuera de ambos (nadie toca): `frontend/tailwind.sh`, `frontend/Dioxus.toml`,
`frontend/AGENTS.md`, `AGENTS.md` (raíz), `README.md`, `docs/ROADMAP.md`,
`docs/INVESTIGACION.md`, `PYMZA.md`, `docker-compose.yml`, `Dockerfile.*`,
`.workflow/**`, `skills/**`, `backend/.env`.

### Tareas

- [ ] T1 (executor-1): motor OCR (tesseract CLI) + endpoints kyc/recibos + score alternativo + fixture + seed hash → brief: `.workflow/briefs/wave5-executor-1.md`
- [ ] T2 (executor-2): panel KYC + score por recibos + badges en frontend → brief: `.workflow/briefs/wave5-executor-2.md`

### Plan de integración

Merges en orden (integrador): **executor-1 (backend) → executor-2 (frontend)**.

```bash
# 0. Precondición humana/entorno: tesseract + spa en el PATH
#    (NixOS: nix run nixpkgs#tesseract -- --version  ||  entorn o shell con tesseract-ocr-spa)

# 1. Build + tests sobre el árbol integrado
cd backend && cargo build && cargo test
cd frontend && cargo check --target wasm32-unknown-unknown && cargo test

# 2. Humo e2e (backend contra Atlas: cd backend && cargo run; demo@pymza.mx / demo1234)
TOKEN=$(curl -s -X POST http://127.0.0.1:3000/api/login \
  -H 'content-type: application/json' \
  -d '{"correo":"demo@pymza.mx","password":"demo1234"}' | jq -r .token)
CURP="GAML930528HDFLNR05"   # del seed
# subir INE de prueba (fixture commiteado, base64):
B64=$(base64 -w0 backend/scripts/fixture_ine.png)
curl -s -X POST http://127.0.0.1:3000/api/clientes/$CURP/kyc -H "Authorization: Bearer $TOKEN" \
  -H 'content-type: application/json' \
  -d "{\"archivo_b64\":\"$B64\",\"mime\":\"image/png\"}" | jq .
# → coincide: true, ine_verificada: true (fixture lleva la CURP del seed)
# subida inválida: mime=text/plain → 400; archivo >2MB → 400
# recibos (dos veces para ver el tope): tipo luz → +25, segundo +25, tercero → 400
curl -s http://127.0.0.1:3000/api/clientes/$CURP -H "Authorization: Bearer $TOKEN" | jq '.cliente | {score, nivel_riesgo, ine_verificada}'

# 3. Humo UI (navegador, humano): subir INE desde la UI → badge; subir recibo →
#    score sube; badges en búsqueda por CURP.
```

Integrador actualiza los estados de la tabla de olas tras cada paso.

### Audit gate

El auditor corre `.workflow/audit-checklist.md` sobre el árbol integrado y
además verifica (evidencia = salida de comandos):

- KYC con fixture → `coincide: true, ine_verificada: true` persiste (curl).
- KYC con CURP distinta (path vs fixture) → `coincide: false`, NO marca
  verificada.
- mime inválido → 400; archivo >2MB → 400; base64 inválido → 400.
- Recibo legible → +25 score; 3er recibo → 400 "Máximo 2"; nivel_riesgo
  recalculado según umbrales del contrato (en vivo con score conocido).
- `buscar_curp`/`buscar_monto` con texto ruidoso OCR → tests unitarios.
- Tesseract ausente → 500 con mensaje claro (simulable con PATH alterado).
- La imagen NO se persiste: no hay colección/blob con la imagen (mongosh
  listado de colecciones).
- Tenant scoping de `recibos` (empresa del token) y tope de 2 por cliente
  global por curp.
- Cero deps nuevas salvo `base64` (git diff Cargo.toml).
- `seed.js` con hash de `demo1234` (login demo funciona: `mongosh < seed.js`
  en DB local de prueba o verificación del hash vs password_correcta).
- Resultado en `.workflow/audits/wave5.md`.

---

## Decision log

Olas 1–4 (contexto histórico; detalle en `.workflow/audits/wave1.md` … `wave4.md`):

| Fecha | Decisión | Por qué |
|------|----------|-----|
| 2026-08-13 | Tenant key = `correo` de empresa; JWT HS256 con `JWT_SECRET` por env; exp 24h | Mínimos que funcionan; techos nombrados |
| 2026-08-13 | App de cobradores y Tauri fuera de este plan | Productos separados; dependen de que la red esté viva |
| 2026-08-13 | OTP por WhatsApp Cloud API (Meta); n8n se reserva para cobranza (ola 7) | Llamada directa del backend al proveedor es lo mínimo |
| 2026-08-17 | VistaPública sin router; auto-login tras registro; default tema dark; `API_BASE` vía `option_env!` | Mínimos que funcionan |
| 2026-08-28 | Re-segmentación 1: identidad / OCR-recibos separadas | Colisiones de archivos impedían paralelismo |
| 2026-08-31 | Re-segmentación 2 → 7 olas; gráficas SVG puro sin librería; registrar pagos como feature raíz de gráficas; gráficas sin datos diferidas | Las gráficas de V exigen datos reales de cobranza |
| 2026-08-31 | Verificación CURP oficial (RENAPO): no hay API pública; vías = convenio o proveedores KYC (Verificamex, JAAK, Truora) | Se integra en ola 7 con trait `VerificadorCurp`; hoy la redundancia es dígito verificador + OTP + OCR INE |
| 2026-09-04 | Ola 3 APPROVED (D1 dv seed, D2 OnceLock) y ola 4 APPROVED (pagos, gráficas, plantilla WhatsApp, TTL) | Auditors en fresco; N1-N3 informativas (huella de humo N3 → owner V) |
| 2026-09-04 | Credenciales demo real: `demo@pymza.mx` / `demo1234` (V re-seedeó); docs actualizadas; seed.js hash viejo → lo actualiza executor-1 ola 5 | Alinea repo con la DB real |

Ola 5 (nuevas):

| Fecha | Decisión | Por qué |
|------|----------|-----|
| 2026-09-04 | Motor OCR = binario `tesseract` como proceso hijo (con timeout 30s), cero deps Rust para el motor | La escalera: ya existe en el sistema (nixpkgs) y el Docker de la ola 6 lo instala; enlazar libtesseract en Rust añade complejidad de build sin valor hoy |
| 2026-09-04 | Subida de archivos = base64 en JSON (no multipart) | Única forma portable con reqwest/wasm; INE/recibo son pequeños (<2MB); trust boundary valida mime + tamaño ANTES de decodificar |
| 2026-09-04 | La imagen NO se guarda (privacy by design) | Solo persiste el resultado (verificación + metadatos); menos datos sensibles en Atlas. Techo: guardar si regulación/negocio lo pide |
| 2026-09-04 | Score por recibos: heurística v1 (+25 por recibo legible, máx 2, nivel por umbrales 750/550) | El score alternativo real exige historial de pagos (open banking, ola 7); la heurística ya da señal usable y es transparente |
| 2026-09-04 | Fixture `fixture_ine.png` commiteado para el humo | El humo de integración no depende de una INE real; determinista y reproducible |
| 2026-09-05 | Ola 5 APPROVED WITH EXCEPTIONS (auditor en fresco): E1 (>2MB → 413 por body-limit de Axum, no el 400 del contrato — handler entra por b64 ≤2MB body; fix con `DefaultBodyLimit` o doc del 413) y E2 (fixture_ine.png produce 40 chars OCR <50 → no sirve para el humo de recibos; flujo de recibos verificado en vivo con imagen sintética: +25, tope 2, nivel recalculado) — ambos owner planner ola 6. N1 (imagen corrupta → 500 "OCR no disponible" en vez de 400) y N2 (tope de recibos sin atomicidad count+insert) informativas | El rechazo >2MB es equivalente en seguridad; el código de recibos funciona end-to-end (evidencia en .workflow/audits/wave5.md). E1/E2 caen en el alcance natural de la ola 6 (tower-http/rate-limit, Docker del humo) |