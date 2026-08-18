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
| Frontend | Dioxus 0.7.9 (pin `=0.7.9`) Rust → WASM + Tailwind. `frontend/AGENTS.md` es la referencia API obligatoria |
| Backend | Axum 0.6 / Tokio. Ya modularizado: `routes/`, `models/`, `auth.rs` |
| DB | MongoDB Atlas (real) vía `MONGODB_URI` en `backend/.env` (gitignored). DB: `pymza`; colecciones: `empresas`, `clientes`, `planes_pago`, `dashboard_stats` |
| Infra | Docker Compose existente; despliegue objetivo: Railway (ola 4) |

Constraints:
- Secretos nunca al repo: `MONGODB_URI`, `JWT_SECRET` solo en `.env` local.
- `Cargo.lock` gitignored (root `.gitignore`) — normal al añadir deps.
- NixOS: `dx` no compila Tailwind; el CSS compilado (`frontend/assets/tailwind.css`) está commiteado. Si una ola cambia clases Tailwind, regenerar con `frontend/tailwind.sh` y commitear el CSS.
- Sin CI. Verificación disponible: `cargo test` (backend y frontend nativo) + `cargo check --target wasm32-unknown-unknown` (frontend WASM).
- Backend necesita librerías dev de OpenSSL para compilar (driver mongodb con `openssl-tls`).
- Release gate (ola 4): `skills/security-audit` con cero CRITICAL/HIGH antes de producción.

## Waves

| Ola | Foco | Estado |
|-----|------|--------|
| 1 | Cimientos: JWT real + aislamiento multi-tenant + frontend partido en módulos | [x] integrada* (merges + build/tests OK; **humo bloqueado**: `backend/.env` JWT_SECRET duplicado — ver decision log 2026-08-17) |
| 2 | Portal público: landing que venda el producto, registro/login separados, sesión persistente pulida, modo claro/oscuro, `API_BASE` configurable | [ ] |
| 3 | Confianza de identidad: validación CURP/correo/teléfono, KYC/OCR real (subida de archivos), score alternativo por recibos de servicios, contrato PDF | [ ] |
| 4 | Producción: despliegue Railway, CORS productivo, rate limiting, backups, security audit (release gate) | [ ] |
| 5 | Dinero: suscripción Stripe (Billing) + dashboard de métricas de impacto | [ ] |
| 6 | Ecosistema: roles inversionista/soporte, buró CdC (sandbox), open banking; cobranza como producto separado | [ ] |

> Estados: planificada → en vuelo → integrada → auditada → hecha.
> Actualizar después de cada paso, quien lo ejecute.

---

## Ola 1 (actual): cimientos multi-tenant

Hoy el producto es una demo single-tenant: token estático `token-temporal-123`
que nadie valida, la empresa se pasa por path parameter sin auth, y cualquier
llamante lee los datos de cualquier empresa. Nada de lo que sigue (portal,
roles, privacidad por tienda, Stripe por empresa) se puede construir encima.

### Contrato API ola 1 (ambos executors implementan contra ESTO)

- Tenant key = `correo` de la empresa (ya único y validado). Los documentos
  nuevos guardan `empresa: <correo>` en `planes_pago` y `dashboard_stats`.
- `POST /api/login` y `POST /api/empresas`: sin cambios de forma. Login sigue
  devolviendo `{status, empresa: nombre_empresa, token}` — pero `token` ahora
  es un JWT real (HS256, claims `sub=<correo>`, `nombre=<nombre_empresa>`,
  `exp=24h`).
- JWT firmado con `JWT_SECRET` (env, obligatoria; el backend arranca con
  error claro si falta).
- Rutas protegidas (Bearer JWT obligatorio; 401 si falta/inválido/expirado):
  `GET /api/clientes/:curp`, `POST /api/clientes`,
  `POST /api/clientes/:curp/reportar`, `POST /api/creditos/evaluar`,
  `POST /api/creditos/autorizar`, `GET /api/creditos`, `GET /api/dashboard`,
  `POST /api/ocr`.
- Cambios de forma:
  - `GET /api/creditos/:empresa` → `GET /api/creditos` (empresa sale del token).
  - `GET /api/dashboard/:empresa` → `GET /api/dashboard` (igual).
  - `POST /api/creditos/autorizar`: el body PIERDE el campo `empresa` (sale del token).
  - `POST /api/clientes/:curp/reportar`: el body PIERDE el campo `empresa` (sale del token).
- Frontend: guarda el token en `localStorage` al entrar, lo lee al arrancar,
  lo borra en logout. 401 → logout automático (ya existe `sesion_ok`).
- Migración de datos existentes: script `backend/scripts/migrate_tenant.js`
  (idempotente) que mapea `planes_pago`/`dashboard_stats` con
  `empresa == nombre_empresa` → `correo` correspondiente en `empresas`.

### Mapa de propiedad de archivos

Dos executors nunca poseen el mismo archivo en la misma ola.

| Archivo/glob | Dueño |
|-----------|-------|
| `backend/Cargo.toml`, `backend/src/**`, `backend/scripts/**`, `docs/API.md`, `.env.example` | executor-1 |
| `frontend/src/**`, `frontend/Cargo.toml` (sin deps nuevas) | executor-2 |

Fuera de ambos (nadie toca): `frontend/tailwind.css`, `frontend/tailwind.sh`,
`frontend/Dioxus.toml`, `frontend/assets/**`, `docker-compose.yml`,
`Dockerfile.*`, `README.md`, `docs/ROADMAP.md`, `PYMZA.md`, `AGENTS.md`,
`.workflow/**`, `backend/.env` (secreto, sin trackear).

### Tareas

- [x] T1 (executor-1): auth JWT real + aislamiento por tenant en el backend → brief: `.workflow/briefs/wave1-executor-1.md` (merge `2532cc8`)
- [x] T2 (executor-2): partir el monolito frontend en módulos + adaptar al contrato API + sesión en localStorage → brief: `.workflow/briefs/wave1-executor-2.md` (merge `aba3ab5`)

### Plan de integración

Orden de merge (integrador): **executor-1 → executor-2** (archivos disjuntos;
el backend define la realidad del contrato).

Comandos sobre el árbol integrado:

```bash
# 1. Build + tests
cd backend && cargo build && cargo test
cd frontend && cargo check --target wasm32-unknown-unknown && cargo test

# 2. Precondición humana: backend/.env con MONGODB_URI (Atlas) + JWT_SECRET
#    (el humano añade JWT_SECRET; el integrador NUNCA lo escribe ni lo imprime)

# 3. Humo end-to-end (backend corriendo contra Atlas: `cd backend && cargo run`)
mongosh < backend/scripts/migrate_tenant.js   # solo si hay datos previos que migrar
TOKEN=$(curl -s -X POST http://127.0.0.1:3000/api/login \
  -H 'content-type: application/json' \
  -d '{"correo":"demo@pymza.mx","password":"demo123"}' | jq -r .token)
test "$TOKEN" != "token-temporal-123" && test "$TOKEN" != "null"
curl -s http://127.0.0.1:3000/api/dashboard -H "Authorization: Bearer $TOKEN" | jq .empresa
curl -s -o /dev/null -w "%{http_code}\n" http://127.0.0.1:3000/api/dashboard          # → 401
curl -s -o /dev/null -w "%{http_code}\n" http://127.0.0.1:3000/api/dashboard \
  -H "Authorization: Bearer token-temporal-123"                                        # → 401

# 4. Humo UI: cd frontend && dx serve → login en navegador → dashboard carga
```

Si la empresa demo no existe en Atlas, el humano decide: seed
(`mongosh < backend/scripts/seed.js`) o usar una cuenta real para el humo.
Integrador actualiza los estados de la tabla de olas aquí tras cada paso.

### Audit gate

El auditor ejecuta `.workflow/audit-checklist.md` sobre el árbol integrado y
además verifica (evidencia = salida de comandos):

- `rg "token-temporal-123"` → 0 hits en el repo.
- `rg "JWT_SECRET|eyJ"` en archivos trackeados → solo `.env.example` con placeholder vacío.
- Las 8 rutas protegidas responden 401 sin token y 200 con token válido (curl por ruta).
- `GET /api/creditos` y `GET /api/dashboard` ya no aceptan path param empresa.
- Un segundo JWT firmado con otro secreto es rechazado (falsificación).
- `frontend/src/main.rs` ya no es el monolito (<200 líneas, módulos en `frontend/src/`).
- `backend/scripts/migrate_tenant.js` existe y es idempotente.
- Resultado escrito en `.workflow/audits/wave1.md`.

---

## Decision log

| Fecha | Decisión | Por qué |
|------|----------|-----|
| 2026-08-13 | Tenant key = `correo` de empresa, no ObjectId nuevo | Ya es único y validado; evita ids nuevos y joins. Docs existentes se migran con script |
| 2026-08-13 | JWT HS256 (`jsonwebtoken` v9), secret por env, exp 24h | Lo mínimo que funciona; techo: refresh tokens + httpOnly cookies si la app lo pide (marcar con `ponytail:`) |
| 2026-08-13 | Frontend se parte en módulos ANTES del portal (ola 2) | El monolito de 1025 líneas impide trabajo paralelo de executors en olas siguientes |
| 2026-08-13 | Contrato API fijado en el plan; executors codifican contra él | Permite backend y frontend en paralelo en la misma ola; la integración verifica el match |
| 2026-08-13 | App de cobradores y Tauri fuera de este plan | Son productos separados; dependen de que la red de crédito esté viva/desplegada |
| 2026-08-13 | `docs/ROADMAP.md` está desactualizado (dice ramas sin merge que ya se mergearon) | Se refresca durante la planificación de la ola 2, no bloquea la ola 1 |
| 2026-08-13 | Atención al hecho de que la alta de empresas SÍ existe en el frontend (form embebido al fondo del Login, `frontend/src/main.rs` ~L209, merged `69e0ad1`) pero sin CTA visible que invite | El usuario la reportó como inexistente por UX; la ola 1 conserva el comportamiento fiel al split; la ola 2 (portal) la convierte en flujo promovido con landing |
| 2026-08-13 | Trabajar directamente contra MongoDB Atlas (solo datos de prueba hasta ahora); el seed demo queda disponible | Aprobado por el usuario; simplifica los humos de integración |
| 2026-08-13 | OTP por WhatsApp: proveedor objetivo = WhatsApp Cloud API (Meta); n8n se descarta para OTP y se reserva para automatización de cobranza (ola 6+) | Para un código de 6 dígitos, una llamada directa del backend al proveedor es lo mínimo que funciona; n8n añade orquestación que no se necesita en el flujo de alta |
| 2026-08-13 | Stripe y Círculo de Crédito: el usuario creará cuentas cuando la ola las pida (5 y 6) | Confirmado por el usuario, presupuesto disponible |
| 2026-08-17 | Integración ola 1: merges `2532cc8` (e1) → `aba3ab5` (e2), orden del plan, sin conflictos. Build + tests del árbol integrado OK (backend 23/23, frontend check wasm + 8/8). **Humo BLOQUEADO por precondición**: `backend/.env` tiene `JWT_SECRET` duplicado — la línea 2 es un placeholder vacío (`JWT_SECRET=""`) que gana la carga de dotenvy y el backend paniquea en `auth.rs:59-61` con "JWT_SECRET está vacía". La línea 3 sí contiene el secreto real (66 chars). NO es fallo del árbol integrado: el código compila y los tests de JWT pasan. Acción humana requerida: borrar la línea vacía (dejar solo el secreto real), relanzar `cd backend && cargo run`, re-correr el humo del plan | El integrador nunca escribe `JWT_SECRET` (plan §Plan de integración); por eso se detiene y reporta en vez de deduplicar el `.env` |
