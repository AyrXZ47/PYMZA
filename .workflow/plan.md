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
| 1 | Cimientos: JWT real + aislamiento multi-tenant + frontend partido en módulos | [x] auditada 2026-08-17 (APPROVED WITH EXCEPTIONS: E1 AGENTS.md falso → ola 2; E2 humo UI navegador pendiente humano) |
| 2 | Portal público: landing que venda, registro/login separados con CTA, modo claro/oscuro, `API_BASE` configurable | [x] integrada 2026-08-28 (build+tests OK, humo e2e OK; humo UI navegador pendiente humano) |
| 3 | Confianza de identidad: validación CURP/correo/teléfono (WhatsApp Cloud API), KYC/OCR real (subida), score alternativo por recibos, contrato PDF | [ ] |
| 4 | Producción: Railway, CORS productivo, rate limiting, backups, security audit (release gate) | [ ] |
| 5 | Dinero: suscripción Stripe (Billing) + dashboard de métricas de impacto | [ ] |
| 6 | Ecosistema: roles inversionista/soporte, buró CdC (sandbox), open banking; cobranza como producto separado | [ ] |

> Estados: planificada → en vuelo → integrada → auditada → hecha.
> Actualizar después de cada paso, quien lo ejecute.

---

## Ola 2 (actual): portal público — la primera impresión vende

Contexto: la ola 1 dejó auth real (JWT, tenant = correo), frontend partido en
módulos (`main.rs` 102 líneas, `api.rs`, `components/*`) y docs inconsistentes
(E1). El problema de producto: al abrir la app el visitante ve una "caja fuerte
con contraseña"; el registro existe pero es invisible como invitación (form
embebido al fondo del Login, decision log 2026-08-13). Un B2B necesita vender
en la primera impresión. Esto desbloquea la ola 3+: todo lo que viene presume
que la empresa se registra sola y entra directo.

### Comportamiento esperado (ambos executors implementan contra ESTO)

- **Sin autenticación** → se ve la **landing** (venta, no login): hero con el
  pitch ("Crédito con cobranza respaldada para tu negocio"), beneficios (score
  con datos alternativos, red de alerta temprana, planes de pago estructurados,
  cartera + dashboard), CTAs "Crear cuenta" e "Iniciar sesión".
- **Registro** = vista propia → form nombre/correo/password ≥8 → `POST
  /api/empresas` → al éxito **auto-login** (`POST /api/login` con las mismas
  credenciales) → entra directo a la app. Enlace cruzado con Login.
- **Login** = solo correo+password; enlace "¿No tienes cuenta? Regístrate".
- **Tema claro/oscuro**: dark por defecto (look actual); toggle 🌙/☀️ en
  sidebar y landing alternando la clase `dark` en `<html>`, persistido en
  localStorage (`pymza_theme`). Migrar componentes a pares base-light +
  `dark:` (Tailwind v4: `@custom-variant dark` en `tailwind.css`). Regenerar y
  commitear `assets/tailwind.css`.
- **`API_BASE` configurable**: `option_env!("API_BASE").unwrap_or("http://127.0.0.1:3000")` — default dev; el build/deploy inyecta la URL real (ola 4, Railway).
- Backend: **cero cambios** (el auto-login usa endpoints existentes).

### Mapa de propiedad de archivos

| Archivo/glob | Dueño |
|-----------|-------|
| `frontend/src/**`, `frontend/tailwind.css`, `frontend/assets/tailwind.css` | executor-1 |
| `AGENTS.md` (raíz), `README.md`, `docs/ROADMAP.md`, `docs/API.md` | executor-2 |

Fuera de ambos (nadie toca): `backend/**`, `frontend/tailwind.sh`,
`frontend/Dioxus.toml`, `frontend/AGENTS.md` (referencia Dioxus),
`docs/INVESTIGACION.md`, `PYMZA.md`, `docker-compose.yml`, `Dockerfile.*`,
`.workflow/**`, `skills/**`, `backend/.env` (secreto).

### Tareas

- [ ] T1 (executor-1): landing + registro/login separados con CTA + tema claro/oscuro + `API_BASE` configurable → brief: `.workflow/briefs/wave2-executor-1.md`
- [ ] T2 (executor-2): refrescar `AGENTS.md` (E1), `README.md`, `docs/ROADMAP.md`, `docs/API.md` al estado real post-ola 1 → brief: `.workflow/briefs/wave2-executor-2.md`

### Plan de integración

Merges en orden (integrador): **executor-1 (frontend) → executor-2 (docs)** —
archivos disjuntos, sin conflictos esperados.

```bash
# 1. Build + tests sobre el árbol integrado
cd backend && cargo build && cargo test    # inalterado; assurance
cd frontend && cargo check --target wasm32-unknown-unknown && cargo test && ./tailwind.sh

# 2. Humo e2e (backend contra Atlas: cd backend && cargo run)
#    Empresa demo: las del humano (p.ej. nueva@empresa.mx / nueva123; JWT_SECRET ya en .env)
TOKEN=$(curl -s -X POST http://127.0.0.1:3000/api/login \
  -H 'content-type: application/json' \
  -d '{"correo":"nueva@empresa.mx","password":"nueva123"}' | jq -r .token)
test "$TOKEN" != "null" && test -n "$TOKEN"
curl -s http://127.0.0.1:3000/api/dashboard -H "Authorization: Bearer $TOKEN" | jq .stats.empresa
curl -s -o /dev/null -w "%{http_code}\n" http://127.0.0.1:3000/api/dashboard   # → 401

# 3. Humo UI (navegador, humano — salda E2 ola 1): cd frontend && dx serve
#    - Sin login: LANDING visible
#    - "Crear cuenta" → Registro → crear empresa de prueba → entra a la app (auto-login)
#    - Logout → Landing/Login; login con la empresa creada → app
#    - Toggle tema: paleta cambia; recarga → se conserva
#    - Flujo completo: alta cliente → evaluar → autorizar → dashboard/cartera
```

Si el humo UI pasa, la excepción E2 de la ola 1 queda saldada. El integrador
actualiza los estados de la tabla de olas.

### Audit gate

El auditor corre `.workflow/audit-checklist.md` sobre el árbol integrado y
además verifica (evidencia = salida de comandos):

- En vivo sin auth: `/` muestra landing, no login.
- Registro crea empresa real en Atlas + auto-login funciona (verificado en vivo).
- `rg "token-temporal-123" frontend/ backend/ AGENTS.md README.md docs/` → 0 hits (E1 saldada).
- `rg ":empresa" frontend/ backend/ AGENTS.md README.md docs/` → 0 (rutas viejas muertas).
- `rg "dark:" frontend/src/` → migración presente; `git diff` de `frontend/assets/tailwind.css` lo refleja (CSS regenerado y commiteado).
- `rg "option_env!\(\"API_BASE\"\)" frontend/src/` → presente.
- Nada fuera del mapa de propiedad modificado (`git log --stat` por rama).
- Resultado en `.workflow/audits/wave2.md`.

---

## Decision log

Ola 1 (contexto histórico; detalle en `.workflow/audits/wave1.md`):

| Fecha | Decisión | Por qué |
|------|----------|-----|
| 2026-08-13 | Tenant key = `correo` de empresa, no ObjectId nuevo | Ya es único y validado; evita ids nuevos y joins. Docs existentes se migran con script |
| 2026-08-13 | JWT HS256 (`jsonwebtoken` v9), secret por env, exp 24h | Lo mínimo que funciona; techo: refresh tokens + httpOnly cookies |
| 2026-08-13 | Frontend se parte en módulos ANTES del portal (ola 2) | El monolito de 1025 líneas impide trabajo paralelo de executors |
| 2026-08-13 | Contrato API fijado en el plan; executors codifican contra él | Permite backend y frontend en paralelo en la misma ola |
| 2026-08-13 | App de cobradores y Tauri fuera de este plan | Productos separados; dependen de que la red esté viva |
| 2026-08-13 | Trabajar contra MongoDB Atlas real (solo datos de prueba) | Aprobado por el usuario; simplifica humos de integración |
| 2026-08-13 | OTP por WhatsApp: proveedor objetivo = WhatsApp Cloud API (Meta); n8n se reserva para cobranza (ola 6+) | Para un código de 6 dígitos, llamada directa del backend al proveedor es lo mínimo que funciona |
| 2026-08-13 | Stripe y CdC: el usuario creará cuentas cuando la ola las pida (5 y 6) | Confirmado por el usuario, presupuesto disponible |
| 2026-08-17 | Humo ola 1 completado tras fix humano del `.env` (JWT_SECRET duplicado, línea vacía ganaba). Aprendizaje: el comando `jq .empresa` del plan lee raíz, pero el schema real entrega `stats.empresa` — corregido en ola 2 | El integrador nunca toca secretos; se detiene y reporta |

Ola 2 (nuevas):

| Fecha | Decisión | Por qué |
|------|----------|-----|
| 2026-08-17 | VistaPública (Landing/Login/Registro) sin router en main.rs | 3 vistas no justifican router; techo: router cuando existan URLs públicas reales (revisitarlo con el portal desplegado, ola 4) |
| 2026-08-17 | Auto-login tras registro exitoso | Contratar sin fricción: alta + sesión directa; el login sigue disponible en logout |
| 2026-08-17 | Default tema = dark (look actual); light opt-in | Minimiza el cambio visual de golpe; el toggle persiste la preferencia |
| 2026-08-17 | `API_BASE` vía `option_env!` con default dev | Configurable en build/deploy sin tocar código (Railway inyectará la URL real en la ola 4); techo: config runtime si se sirve desde otro origen |
| 2026-08-17 | E1 (AGENTS.md falso) resuelta como executor de docs en la ola 2 | La auditoría la marcó con owner "planner ola 2"; docs = zona disjunta que da paralelismo al frontend |
| 2026-08-17 | Backend no cambia en la ola 2 | El auto-login y la landing no requieren endpoints nuevos; menos riesgo, menos diff |