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
| Frontend | Dioxus 0.7.9 (pin `=0.7.9`) Rust → WASM + Tailwind v4. `frontend/AGENTS.md` es la referencia API obligatoria. `API_BASE` configurable por build (ola 2) |
| Backend | Axum 0.6 / Tokio. Modularizado: `routes/`, `models/`, `auth.rs`, `otp.rs`, `ocr.rs` |
| DB | MongoDB Atlas (real) vía `MONGODB_URI`. Colecciones: `empresas`, `clientes`, `planes_pago`, `dashboard_stats`, `verificaciones`, `pagos`, `recibos` |
| Infra | Docker Compose + Dockerfiles (tesseract en imagen backend); despliegue objetivo: Railway (esta ola, ejecutado por V tras el release gate) |

Constraints:
- Secretos nunca al repo: `MONGODB_URI`, `JWT_SECRET`, credenciales de proveedores (WhatsApp, Stripe, CdC, KYC) solo en `.env` local / variables de Railway.
- `Cargo.lock` gitignored — normal al añadir deps.
- NixOS: `dx` no compila Tailwind; el CSS compilado está commiteado. Regenerar con `frontend/tailwind.sh` si una ola cambia clases.
- Sin CI. Verificación: `cargo test` + `cargo check --target wasm32-unknown-unknown`.
- OCR: binario `tesseract` (Docker: `tesseract-ocr` + `tesseract-ocr-spa`).
- **Release gate (ESTA ola): `skills/security-audit` con cero CRITICAL/HIGH (o excepciones documentadas con owner) ANTES de desplegar a Railway.**
- Demo real en Atlas: `demo@pymza.mx` / `demo1234`.

## Waves

| Ola | Foco | Estado |
|-----|------|--------|
| 1 | Cimientos: JWT real + multi-tenant + frontend modularizado | [x] auditada 2026-08-17 |
| 2 | Portal público: landing, registro/login, tema claro/oscuro, `API_BASE` configurable | [x] auditada 2026-08-28 |
| 3 | Identidad verificable: CURP dv, correo, OTP teléfono (WhatsApp/mock) | [x] auditada 2026-08-31 |
| 4 | Cartera viva: pagos + estados de plan + gráficas SVG + favicon | [x] auditada 2026-09-04 |
| 5 | KYC/OCR real (tesseract) + score alternativo por recibos | [x] auditada 2026-09-05 (APPROVED WITH EXCEPTIONS: E1 413→ola 6, E2 fixture→ola 6) |
| 6 | Contrato PDF + Producción: CORS productivo, body limit, rate limiting, Dockerfiles Railway, security audit (release gate) | [x] integrada 2026-09-06 (merges + cargo build/test OK, push main) — humo Docker PENDIENTE (socket docker requiere reinicio para aplicar grupo) |
| 7 | Dinero (Stripe) + Ecosistema: roles, verificación CURP oficial (proveedor RENAPO), buró CdC (sandbox), open banking | [ ] |

> Estados: planificada → en vuelo → integrada → auditada → hecha.
> Actualizar después de cada paso, quien lo ejecute.

---

## Ola 6 (actual): contrato PDF + producción (release gate)

Contexto: la ola 5 quedó APPROVED WITH EXCEPTIONS (E1: archivos >2MB devuelven
413 por el body-limit de Axum en lugar del 400 del contrato; E2: el fixture
no sirve para el humo de recibos) — ambas caen en el alcance de esta ola.
Esta ola convierte el proyecto en un **producto desplegable**: el PDF del
contrato que la empresa le entrega al cliente, el hardening pre-producción
(CORS, límites, rate limiting), los Dockerfiles listos para Railway, y el
**release gate**: `skills/security-audit` con cero CRITICAL/HIGH antes de que
V despliegue.

División clara de responsabilidades: **los executors dejan TODO listo y
verificado (build Docker local incluido); el DESPLIEGUE a Railway lo ejecuta
V con `docs/DEPLOY.md` DESPUÉS de que la auditoría apruebe el release gate.**

### Contrato API ola 6 (ambos executors implementan contra ESTO)

- **Contrato PDF**:
  - `GET /api/creditos/{plan_id}/contrato` (protegido): genera y devuelve el
    PDF del plan (Content-Type: application/pdf). Datos: nombre/correo de la
    empresa (lookup `empresas` por el correo del token), nombre/CURP del
    cliente, producto, monto total, plazo, tasa, tabla completa de pagos
    (mes, pago, interés, capital, saldo), fecha de emisión, línea de firma y
    leyenda mínima ("Contrato de crédito generado por PYMZA"). Plan ajeno al
    tenant → 404. Solo planes del token.
  - Motor: crate `printpdf` (pura Rust, sin deps de sistema), fuente
    Helvetica base14 (latin1 — acentos OK). Generación como función pura
    `pdf_contrato(empresa, cliente, plan) -> Vec<u8>` testeada (header
    `%PDF`, tamaño mínimo).
  - Dep nueva backend: SOLO `printpdf`.
- **Hardening backend**:
  - **CORS productivo**: `cors_layer` lee `ALLOWED_ORIGINS` (env,
    separada por comas) con default dev actual
    (`http://localhost:8080,http://127.0.0.1:8080`) — el techo que la ola 1
    ya documentó.
  - **Body limit**: `DefaultBodyLimit::max(3_000_000)` en el Router (cierra
    E1: el handler de kyc/recibos vuelve a ser quien rechace >2MB con el 400
    del contrato; el límite global de 3MB es la red de seguridad para el b64).
  - **Rate limiting por IP en las rutas públicas** (`/api/login`,
    `/api/empresas`): crate `tower-governor` (dep nueva única de esta pieza)
    — p. ej. 10 req/60s por IP; error 429 con mensaje claro. Rutas protegidas
    NO (ya exigen JWT). Evidencia en tests: 11ª petición → 429.
- **Dockerfiles para Railway**:
  - `Dockerfile.backend`: añadir `tesseract-ocr` + `tesseract-ocr-spa`
    (E1/E2 humo) y asegurar envs (`BIND_ADDR=0.0.0.0:3000` ya está).
  - `Dockerfile.frontend`: `ARG API_BASE` (default `http://127.0.0.1:3000`)
    → `ENV API_BASE` antes del build WASM (la ola 2 preparó `option_env!`;
    ahora se consume en build). `docker-compose.yml`: servicio frontend con
    `args: API_BASE` (SOLO el servicio frontend).
  - El humo de integración: `docker compose build` + contenedores corriendo
    localmente contra Atlas (login real desde el frontend servido por Docker).
- **Backups**: sin código — MongoDB Atlas los trae (backups automáticos del
  cluster); `docs/DEPLOY.md` documenta cómo verificarlos en la UI de Atlas
  (owner V al desplegar).
- **Frontend**:
  - Botón "Descargar contrato" en cada plan de cartera (y en el modal al
    autorizar, opcional): descarga el PDF y dispara el download del
    navegador (blob URL vía web_sys). `api.rs`:
    `descargar_contrato(plan_id, token) -> Vec<u8>` + helper de download.
  - Test del helper en host (payload → download event simulable o al menos
    parseo del Content-Type/bytes).
- **docs/DEPLOY.md** (nuevo): guía paso a paso de Railway para V — servicios
  (backend: repo+Dockerfile, envs `MONGODB_URI`, `JWT_SECRET`,
  `WHATSAPP_TOKEN`, `WHATSAPP_PHONE_NUMBER_ID`, `WHATSAPP_TEMPLATE`,
  `WHATSAPP_TEMPLATE_LANG`, `ALLOWED_ORIGINS=<dominio frontend>`,
  `OCR_LANG=spa`; frontend: build arg `API_BASE=<url pública del backend>`),
  dominio/puerto, cómo verificar backups de Atlas, cómo girar `JWT_SECRET`
  (logout masivo), y troubleshooting. Cero valores reales de secretos.
- **docs/API.md** + **.env.example** (`ALLOWED_ORIGINS`).
- **E2 del auditor ola 5**: añadir `backend/scripts/fixture_recibo.png`
  (imagen legible con monto, `TOTAL: $450.00 MXN` estilo fixture_ine; el
  executor-1 verifica con tesseract que el OCR la lee y el parser extrae el
  monto). El humo de recibos de la próxima integración usa este fixture.

### Mapa de propiedad de archivos

| Archivo/glob | Dueño |
|-----------|-------|
| `backend/Cargo.toml`, `backend/src/**`, `backend/scripts/**`, `docs/API.md`, `.env.example`, `Dockerfile.backend` | executor-1 |
| `frontend/src/**`, `frontend/tailwind.css`, `frontend/assets/**`, `frontend/Cargo.toml` (si hiciera falta, sin deps nuevas), `Dockerfile.frontend`, `docker-compose.yml` (SOLO servicio `frontend`: build args), `docs/DEPLOY.md`, `README.md` (SOLO añadir enlace a DEPLOY.md) | executor-2 |

Fuera de ambos (nadie toca): `frontend/tailwind.sh`, `frontend/Dioxus.toml`,
`frontend/AGENTS.md`, `AGENTS.md` (raíz), `docs/ROADMAP.md`,
`docs/INVESTIGACION.md`, `docs/API.md` (dueño: executor-1), `PYMZA.md`,
`Dockerfile.backend` (dueño: executor-1), el resto de servicios de
`docker-compose.yml`, `.workflow/**`, `skills/**`, `backend/.env`.

### Tareas

- [ ] T1 (executor-1): contrato PDF + CORS env + body limit + rate limit + fixture_recibo + Dockerfile.backend → brief: `.workflow/briefs/wave6-executor-1.md`
- [ ] T2 (executor-2): descarga de contrato + Dockerfile.frontend (API_BASE) + compose args + DEPLOY.md → brief: `.workflow/briefs/wave6-executor-2.md`

### Plan de integración

Merges en orden (integrador): **executor-1 (backend) → executor-2 (frontend)**.

```bash
# 1. Build + tests sobre el árbol integrado
cd backend && cargo build && cargo test
cd frontend && cargo check --target wasm32-unknown-unknown && cargo test && ./tailwind.sh

# 2. Humo Docker (pre-Railway): los contenedores construyen y funcionan en local
docker compose build
docker compose up -d mongo
# backend en contenedor con .env del host: MONGODB_URI/JWT_SECRET por env del compose
docker compose up backend && docker compose up -d frontend
docker run --rm $(docker compose ps -q backend) tesseract --version   # OCR presente
TOKEN=$(curl -s -X POST http://127.0.0.1:3000/api/login \
  -H 'content-type: application/json' \
  -d '{"correo":"demo@pymza.mx","password":"demo1234"}' | jq -r .token)
# contrato PDF de un plan del tenant (tomar _id de GET /api/creditos)
PLAN_ID=$(curl -s http://127.0.0.1:3000/api/creditos -H "Authorization: Bearer $TOKEN" | jq -r '.creditos[0]._id')
curl -s http://127.0.0.1:3000/api/creditos/$PLAN_ID/contrato -H "Authorization: Bearer $TOKEN" -o /tmp/contrato.pdf && head -c 8 /tmp/contrato.pdf   # → %PDF-
curl -s http://127.0.0.1:3000/api/creditos/$PLAN_ID/contrato -o /dev/null -w "%{http_code}\n"                                                        # → 401
# rate limit: 11 logins seguidos → el último devuelve 429
# body limit: subir base64 de 2.5MB real → 400 con mensaje del contrato (E1 cerrada)
docker compose down

# 3. Humo UI (navegador, humano): login servido por Docker, botón "Descargar
#    contrato" baja un PDF válido y legible con la tabla de pagos.
```

Integrador actualiza los estados de la tabla de olas tras cada paso.

### Audit gate (RELEASE GATE)

El auditor corre `.workflow/audit-checklist.md` sobre el árbol integrado y,
**por ser la ola pre-despliegue, corre además `skills/security-audit`
(6 fases) sobre el árbol integrado**:

- Veredicto del security-audit: cero CRITICAL/HIGH, o excepciones documentadas
  con owner en `.workflow/audits/wave6.md`.
- `GET /api/creditos/{plan_id}/contrato`: 200 `%PDF` con datos del tenant;
  plan ajeno → 404; sin token → 401.
- CORS: `ALLOWED_ORIGINS` respetado (curl con `Origin` fuera de la lista →
  sin header de allow; dentro → header presente); default dev intacto.
- E1 cerrada: archivo real >2MB → **400** con el mensaje del contrato (no 413).
- Rate limit: 11º login desde la misma IP → 429 (config por env, no hardcode).
- Docker: `docker compose build` OK; `tesseract --version` dentro de la
  imagen backend; frontend construido con `API_BASE` inyectado (grep del
  binario wasm o verificación de que el contenedor sirve y loguea).
- `fixture_recibo.png`: tesseract lo lee y `buscar_monto` extrae 450.00
  (test + comando) — E2 cerrada.
- `docs/DEPLOY.md` sin secretos reales (scan) y con las envs listadas.
- `ponytail:` comments donde correspondan; cero deps nuevas salvo
  `printpdf` y `tower-governor` (git diff Cargo.toml).
- Humo UI en navegador (owner V) queda anotado; NO bloquea el despliegue si
  V lo valida al probar Railway.

---

## Decision log

Olas 1–5 (contexto histórico; detalle en `.workflow/audits/wave1.md` … `wave5.md`):

| Fecha | Decisión | Por qué |
|------|----------|-----|
| 2026-08-13 | Tenant key = `correo` de empresa; JWT HS256 con `JWT_SECRET` por env; exp 24h | Mínimos que funcionan; techos nombrados |
| 2026-08-13 | App de cobradores y Tauri fuera de este plan | Productos separados |
| 2026-08-13 | OTP por WhatsApp Cloud API (Meta); n8n se reserva para cobranza (ola 7) | Mínimo que funciona |
| 2026-08-17 | VistaPública sin router; auto-login; default tema dark; `API_BASE` vía `option_env!` | Mínimos que funcionan |
| 2026-08-28 | Re-segmentación 1: identidad / OCR-recibos separadas | Colisiones de archivos |
| 2026-08-31 | Re-segmentación 2 → 7 olas; SVG puro; registrar pagos como feature raíz de gráficas; verificación RENAPO vía proveedor (ola 7) | Datos reales > decoración |
| 2026-09-04/05 | Olas 3-4 APPROVED; ola 5 APPROVED WITH EXCEPTIONS (E1 413, E2 fixture → owners planner ola 6) | Auditorías en fresco con evidencia |
| 2026-09-04 | Motor OCR = binario tesseract; subida base64 en JSON; imagen no persistida; score recibos heurística v1 (+25, máx 2) | Ponytail con techos nombrados |

Ola 6 (nuevas):

| Fecha | Decisión | Por qué |
|------|----------|-----|
| 2026-09-05 | Contrato PDF con `printpdf` + Helvetica base14 (latin1: acentos OK), bajo demanda (`GET por plan_id`) | Pura Rust sin deps de sistema; el PDF se regenera siempre desde datos vivos, no se almacena. Techo: firma electrónica/logo si el negocio lo pide |
| 2026-09-05 | CORS por env `ALLOWED_ORIGINS` (default dev) | El techo documentado desde la ola 1; Railway inyecta el dominio del frontend |
| 2026-09-05 | `DefaultBodyLimit::max(3MB)` global — cierra E1 (413→400 del contrato) | El handler vuelve a ser quien rechaza con el 400 y mensaje del contrato; el límite global queda como red de seguridad |
| 2026-09-05 | Rate limit por IP (tower-governor) SOLO en rutas públicas (login, empresas) | Las protegidas ya exigen JWT; el brute-force solo es posible en las públicas. Techo: extender a OTP si se abusa |
| 2026-09-05 | Backups = feature de Atlas (sin código); DEPLOY.md documenta la verificación | No reimplementar lo que el proveedor trae |
| 2026-09-05 | El despliegue a Railway lo ejecuta V con `docs/DEPLOY.md` DESPUÉS del release gate | V tiene la cuenta y las credenciales; el release gate (security audit) corre antes de exponer nada |
| 2026-09-05 | E2 cerrada con `fixture_recibo.png` nuevo | El humo de recibos queda reproducible sin imagen sintética ad-hoc |