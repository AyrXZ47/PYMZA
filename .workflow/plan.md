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
| Backend | Axum 0.6 / Tokio. Modularizado: `routes/`, `models/`, `auth.rs` (JWT HS256) |
| DB | MongoDB Atlas (real) vía `MONGODB_URI` en `backend/.env` (gitignored). DB: `pymza`; colecciones: `empresas`, `clientes`, `planes_pago`, `dashboard_stats` (+ `verificaciones` desde ola 3) |
| Infra | Docker Compose existente; despliegue objetivo: Railway (ola 5) |

Constraints:
- Secretos nunca al repo: `MONGODB_URI`, `JWT_SECRET`, credenciales de proveedores (WhatsApp, Stripe, CdC) solo en `.env` local.
- `Cargo.lock` gitignored (root `.gitignore`) — normal al añadir deps.
- NixOS: `dx` no compila Tailwind; el CSS compilado (`frontend/assets/tailwind.css`) está commiteado. Si una ola cambia clases Tailwind, regenerar con `frontend/tailwind.sh` y commitear el CSS.
- Sin CI. Verificación disponible: `cargo test` (backend y frontend nativo) + `cargo check --target wasm32-unknown-unknown` (frontend WASM).
- Backend necesita librerías dev de OpenSSL para compilar (driver mongodb con `openssl-tls`).
- Release gate (ola 5): `skills/security-audit` con cero CRITICAL/HIGH antes de producción.

## Waves

| Ola | Foco | Estado |
|-----|------|--------|
| 1 | Cimientos: JWT real + aislamiento multi-tenant + frontend partido en módulos | [x] auditada 2026-08-17 (APPROVED WITH EXCEPTIONS; E1 saldada ola 2, E2 saldada 2026-08-28) |
| 2 | Portal público: landing que venda, registro/login separados con CTA, modo claro/oscuro, `API_BASE` configurable | [x] auditada 2026-08-28 (APPROVED WITH EXCEPTIONS; E1/E2 resueltas, attest V) |
| 3 | Identidad verificable: CURP robusta (dígito verificador), correo del cliente, verificación por teléfono OTP (WhatsApp Cloud API, mock en dev) | [x] auditada 2026-08-31 (APPROVED; dv verificado vs DOF, hash OTP en vivo; O1-O4 no bloqueantes en `.workflow/audits/wave3.md`) |
| 4 | KYC/OCR real (subida de archivos) + score alternativo por recibos de servicios | [ ] |
| 5 | Contrato PDF + Producción: Railway, CORS productivo, rate limiting, backups, security audit (release gate) | [ ] |
| 6 | Dinero (Stripe) + Ecosistema: roles inversionista/soporte, buró CdC (sandbox), open banking; cobranza como producto separado | [ ] |

> Nota de re-segmentación (decision log 2026-08-28): la ola 3 original agrupaba
> 4 features grandes que colisionaban en `main.rs`/`Cargo.toml`/`models/cliente.rs`;
> se repartieron en olas 3 y 4 para mantener el paralelismo con archivos disjuntos.

> Estados: planificada → en vuelo → integrada → auditada → hecha.
> Actualizar después de cada paso, quien lo ejecute.

---

## Ola 3 (actual): identidad verificable

Contexto: el alta de cliente valida CURP solo por formato (largo y patrones),
no por dígito verificador; no pide correo; y el teléfono se captura sin
verificar que sea del cliente real. La idea (PYMZA.md) pide redundancia:
CURP real, teléfono con código, correo real. Esta ola hace la verificación
fuerte por **teléfono + código OTP** (WhatsApp Cloud API; mock en dev que
imprime el código en el log del backend), la **CURP con dígito verificador**,
y añade el **correo del cliente** (campo + formato; el envío de código por
correo queda diferido, ponytail: ver decision log).

### Contrato API ola 3 (ambos executors implementan contra ESTO)

- `Cliente` gana dos campos:
  - `correo: Option<String>` (opcional; formato validado con `es_correo_valido`).
  - `telefono_verificado: bool` (default `false`).
  - `CrearClienteReq` gana `correo: Option<String>`.
- `POST /api/clientes` (protegido): crea el cliente con `telefono_verificado:
  false`; valida CURP robusta (formato + dígito verificador + coherencia
  fecha/sexo/entidad) y correo si viene.
- Nuevo módulo `backend/src/routes/verificacion.rs` (2 rutas protegidas):
  - `POST /api/verificaciones/solicitar` — body `{ curp, telefono }`: genera
    código de 6 dígitos, expira en 10 min, guarda el desafío en colección
    `verificaciones` (SOLO el hash del código, nunca en claro), lo envía por
    el `OtpSender` activo. Respuesta `{status: "success"}`. El mock imprime
    el código en el log del backend (para el flujo de dev).
  - `POST /api/verificaciones/confirmar` — body `{ curp, telefono, codigo }`:
    valida contra el desafío vigente (no expirado), marca
    `telefono_verificado = true` en el cliente de esa CURP, borra el desafío.
    Errores: 400 código inválido/expirado, 404 sin desafío o cliente inexistente.
- `OtpSender`: trait con dos impls — `MockOtpSender` (default, imprime el
  código en el log) y `WhatsAppOtpSender` (llama a la WhatsApp Cloud API de
  Meta si `WHATSAPP_TOKEN` y `WHATSAPP_PHONE_NUMBER_ID` existen en env; usa
  `reqwest`). Selector en `main.rs` por env. Cero secretos en el repo.
- `backend/src/otp.rs`: generación de código, hash, expiración, trait e impls.
- Deps nuevas backend (justificadas): `reqwest` (llamar WhatsApp API),
  `rand` (código), `sha2` (hash del código). Ninguna otra.
- Frontend `alta_cliente.rs`: tras dar de alta un cliente, sección "Verificar
  teléfono" — botón "Enviar código" → input de 6 dígitos → "Confirmar" →
  badge "✓ Verificado". En el resultado de búsqueda por CURP, mostrar badge
  si `telefono_verificado` es true. `api.rs`: helpers
  `solicitar_verificacion` y `confirmar_verificacion`.
- Backend: **cero cambios** en el resto de rutas/contratos existentes.

### Mapa de propiedad de archivos

| Archivo/glob | Dueño |
|-----------|-------|
| `backend/Cargo.toml`, `backend/src/**`, `backend/scripts/**`, `docs/API.md`, `.env.example` | executor-1 |
| `frontend/src/**`, `frontend/tailwind.css`, `frontend/assets/tailwind.css` | executor-2 |

Fuera de ambos (nadie toca): `frontend/tailwind.sh`, `frontend/Dioxus.toml`,
`frontend/AGENTS.md` (referencia Dioxus), `frontend/Cargo.toml` (cero deps),
`AGENTS.md` (raíz), `README.md`, `docs/ROADMAP.md`, `docs/INVESTIGACION.md`,
`PYMZA.md`, `docker-compose.yml`, `Dockerfile.*`, `.workflow/**`,
`skills/**`, `backend/.env` (secreto).

### Tareas

- [x] T1 (executor-1): CURP robusta + correo del cliente + OTP teléfono (WhatsApp/mock) → brief: `.workflow/briefs/wave3-executor-1.md`
- [x] T2 (executor-2): flujo de verificación de teléfono en alta de cliente + badges → brief: `.workflow/briefs/wave3-executor-2.md`

### Plan de integración

Merges en orden (integrador): **executor-1 (backend) → executor-2 (frontend)**.

```bash
# 1. Build + tests sobre el árbol integrado
cd backend && cargo build && cargo test
cd frontend && cargo check --target wasm32-unknown-unknown && cargo test

# 2. Humo e2e (backend contra Atlas: cd backend && cargo run)
TOKEN=$(curl -s -X POST http://127.0.0.1:3000/api/login \
  -H 'content-type: application/json' \
  -d '{"correo":"nueva@empresa.mx","password":"nueva123"}' | jq -r .token)
# alta de cliente de prueba (CURP con dígito verificador válido)
CURP="GACM940101HDFRRR07"
curl -s -X POST http://127.0.0.1:3000/api/clientes -H "Authorization: Bearer $TOKEN" \
  -H 'content-type: application/json' \
  -d "{\"curp\":\"$CURP\",\"nombre_completo\":\"Prueba OTP\",\"direccion\":\"X\",\"telefono\":\"5512345678\",\"correo\":\"prueba@correo.mx\"}" | jq .
# solicitar código → buscar el código en el LOG del backend (mock)
curl -s -X POST http://127.0.0.1:3000/api/verificaciones/solicitar -H "Authorization: Bearer $TOKEN" \
  -H 'content-type: application/json' -d "{\"curp\":\"$CURP\",\"telefono\":\"5512345678\"}" | jq .
# confirmar con el código del log
curl -s -X POST http://127.0.0.1:3000/api/verificaciones/confirmar -H "Authorization: Bearer $TOKEN" \
  -H 'content-type: application/json' -d "{\"curp\":\"$CURP\",\"telefono\":\"5512345678\",\"codigo\":\"<del-log>\"}" | jq .
# GET /api/clientes/$CURP → telefono_verificado: true
curl -s http://127.0.0.1:3000/api/clientes/$CURP -H "Authorization: Bearer $TOKEN" | jq .

# 3. Humo UI (navegador, humano): alta cliente → enviar código → confirmar →
#    badge verificado; búsqueda muestra badge.
```

Integrador actualiza los estados de la tabla de olas tras cada paso.

### Audit gate

El auditor corre `.workflow/audit-checklist.md` sobre el árbol integrado y
además verifica (evidencia = salida de comandos):

- CURP robusta: `cargo test` incluye casos con dígito verificador inválido
  (formato OK pero dígito malo → rechazada).
- El código OTP NUNCA se guarda en claro: `rg "codigo_hash" backend/src/`
  presente; la colección `verificaciones` solo tiene hash (verificado en vivo
  con `mongosh` o log).
- Sin token → 401 en `solicitar`/`confirmar` (curl).
- Con token pero código inválido/expirado → 400; sin desafío → 404.
- `telefono_verificado` persiste en el cliente tras confirmar (curl GET).
- Cero secretos commiteados (`rg "WHATSAPP_" AGENTS.md README.md docs/ backend/src/`
  → solo `.env.example` con placeholder vacío).
- Patrón exacto de rutas viejas (aprendizaje ola 2: NO usar `:empresa` suelto,
  usar `/api/creditos/:empresa` literal): 0 hits en `backend/src/`.
- Nada fuera del mapa de propiedad modificado (`git log --stat` por rama).
- Resultado en `.workflow/audits/wave3.md`.

---

## Decision log

Olas 1–2 (contexto histórico; detalle en `.workflow/audits/wave1.md` y
`wave2.md`):

| Fecha | Decisión | Por qué |
|------|----------|-----|
| 2026-08-13 | Tenant key = `correo` de empresa, no ObjectId nuevo | Ya es único y validado; docs existentes se migran con script |
| 2026-08-13 | JWT HS256 (`jsonwebtoken` v9), secret por env, exp 24h | Lo mínimo que funciona; techo: refresh tokens + httpOnly cookies |
| 2026-08-13 | App de cobradores y Tauri fuera de este plan | Productos separados; dependen de que la red esté viva |
| 2026-08-13 | OTP por WhatsApp: proveedor objetivo = WhatsApp Cloud API (Meta); n8n se reserva para cobranza (ola 6+) | Para un código de 6 dígitos, llamada directa del backend al proveedor es lo mínimo que funciona |
| 2026-08-17 | Humo ola 1: el schema real entrega `stats.empresa`, no `empresa` | Corregido el comando del plan en olas siguientes |
| 2026-08-17 | VistaPública sin router en la ola 2 | 3 vistas no justifican router; techo: router cuando existan URLs públicas reales (ola 5) |
| 2026-08-17 | Auto-login tras registro exitoso | Contratar sin fricción: alta + sesión directa |
| 2026-08-17 | Default tema = dark; light opt-in | Minimiza el cambio visual de golpe |
| 2026-08-17 | `API_BASE` vía `option_env!` con default dev | Configurable en build/deploy sin tocar código |
| 2026-08-28 | E1 (AGENTS.md falso) y E2 (humo UI) de la ola 2 resueltas: stash aplicado (`f352e38`), attest navegador de V | Árbol sin stashes, E2 heredada de ola 1 saldada |

Ola 3 (nuevas):

| Fecha | Decisión | Por qué |
|------|----------|-----|
| 2026-08-28 | Re-segmentación: ola 3 = identidad (CURP+correo+OTP); ola 4 = KYC/OCR + score por recibos; ola 5 = contrato PDF + producción | Las 4 features originales de la "ola 3" colisionaban en `main.rs`/`Cargo.toml`/`models/cliente.rs`; partidas, cada ola mantiene 2 executors con archivos disjuntos |
| 2026-08-28 | Código OTP: 6 dígitos, hash SHA-256 en DB (nunca en claro), expira en 10 min, ligado a `curp+telefono` | El desafío verifica que el teléfono pertenece al cliente que se da de alta; hash protege la DB si se filtra |
| 2026-08-28 | `OtpSender` trait: `MockOtpSender` (default, código en log) y `WhatsAppOtpSender` (env `WHATSAPP_TOKEN`+`WHATSAPP_PHONE_NUMBER_ID`) | Dev sin credenciales funciona; producción solo activa WhatsApp con env. Cero secretos en repo |
| 2026-08-28 | Correo del cliente: campo + formato ahora; envío de código por correo diferido | El OTP por WhatsApp es la verificación fuerte; el correo con código exige SMTP — se añade cuando haya un proveedor (ponytail: techo nombrado) |
| 2026-08-28 | Deps nuevas backend permitidas: `reqwest`, `rand`, `sha2` | Las tres cubren el OTP (HTTP al proveedor, código, hash); ninguna otra |
| 2026-08-31 | Integración ola 3: la CURP de ejemplo del plan (`GACM940101HDFRRR07`) NO pasa el dígito verificador; usar CURPs del seed (`RAMJ920215MDFMZR05`, etc.) | El ejemplo se escribió antes de implementar el verificador; los tests cubren ambas vías |