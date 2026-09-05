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
| DB | MongoDB Atlas (real) vía `MONGODB_URI` en `backend/.env` (gitignored). DB: `pymza`; colecciones: `empresas`, `clientes`, `planes_pago`, `dashboard_stats`, `verificaciones` (+ `pagos` desde ola 4) |
| Infra | Docker Compose existente; despliegue objetivo: Railway (ola 6) |

Constraints:
- Secretos nunca al repo: `MONGODB_URI`, `JWT_SECRET`, credenciales de proveedores (WhatsApp, Stripe, CdC, KYC) solo en `.env` local.
- `Cargo.lock` gitignored (root `.gitignore`) — normal al añadir deps.
- NixOS: `dx` no compila Tailwind; el CSS compilado (`frontend/assets/tailwind.css`) está commiteado. Si una ola cambia clases Tailwind, regenerar con `frontend/tailwind.sh` y commitear el CSS.
- Sin CI. Verificación disponible: `cargo test` (backend y frontend nativo) + `cargo check --target wasm32-unknown-unknown` (frontend WASM).
- Backend necesita librerías dev de OpenSSL para compilar (driver mongodb con `openssl-tls`).
- Release gate (ola 6): `skills/security-audit` con cero CRITICAL/HIGH antes de producción.

## Waves

| Ola | Foco | Estado |
|-----|------|--------|
| 1 | Cimientos: JWT real + aislamiento multi-tenant + frontend partido en módulos | [x] auditada 2026-08-17 |
| 2 | Portal público: landing que venda, registro/login separados, tema claro/oscuro, `API_BASE` configurable | [x] auditada 2026-08-28 |
| 3 | Identidad verificable: CURP con dígito verificador, correo del cliente, OTP teléfono (WhatsApp/mock) | [x] auditada 2026-08-31 (APPROVED; D1/D2 aprobadas; O1-O3 observaciones con owner) |
| 4 | Cartera viva: registro de pagos + estados de plan + gráficas de impacto (SVG) + favicon | [x] integrada 2026-09-04 (build+tests verdes; auditoría pendiente) |
| 5 | KYC/OCR real (subida de archivos) + score alternativo por recibos de servicios | [ ] |
| 6 | Contrato PDF + Producción: Railway, CORS productivo, rate limiting, backups, security audit (release gate) | [ ] |
| 7 | Dinero (Stripe) + Ecosistema: roles, verificación CURP oficial (proveedor RENAPO), buró CdC (sandbox), open banking | [ ] |

> Re-segmentaciones documentadas en el decision log (2026-08-28 y 2026-08-31).
> Estados: planificada → en vuelo → integrada → auditada → hecha.
> Actualizar después de cada paso, quien lo ejecute.

---

## Ola 4 (actual): cartera viva — pagos, gráficas de impacto, favicon

Contexto: la ola 3 quedó APPROVED. V pidió (2026-08-31): gráficas de impacto
bajo los KPIs del dashboard (captura con 3 tiers de gráficas) y favicon/logo
en la pestaña del navegador.

**Hallazgo de planificación**: hoy el sistema NO registra pagos —
`autorizar` inserta el plan con `estado: "Activo"` fijo y jamás cambia. Sin
pagos registrados, "cobrado" siempre es 0 y la morosidad es pura proyección:
las gráficas de V dirían mentiras. Por eso la feature raíz de esta ola es el
**registro de pagos** (el corazón del pitch PYMZA es la cobranza); las
gráficas son su consecuencia visible.

### Contrato API ola 4 (ambos executors implementan contra ESTO)

- **Colección `pagos`** (nueva): `{ plan_id (ObjectId del plan, como hex),
  empresa (correo del token), cliente_curp, cuota (1..=plazo_meses), monto,
  fecha (UTC) }`.
- **`POST /api/creditos/pagos`** (protegido): body `{ plan_id, cuota, monto }`.
  Valida: plan existe y `plan.empresa == correo del token`; cuota en
  `1..=plazo_meses`; cuota no pagada ya (400 si duplicada); monto igual a
  `pago_mensual` del plan con tolerancia de 1 centavo (400 con mensaje claro).
  Inserta el pago, recalcula el estado del plan y responde
  `{status: "success", plan: {...}}`.
- **Estados del plan** (recalculados tras cada pago): `Liquidado` (todas las
  cuotas pagadas), `Moroso` (≥1 cuota vencida no pagada: vencimiento de la
  cuota n = `fecha` del plan + n meses, antes de hoy), `Activo` (resto).
  `autorizar` sigue creando con "Activo".
- **`GET /api/creditos`** (existe): cada plan gana `cuotas_pagadas` y
  `cuotas_vencidas` (calculadas en servidor) para que el frontend solo dibuje.
- **`GET /api/creditos/resumen`** (nuevo, protegido, tenant del token):
  `{status: "success", resumen: {...}}` con EXACTAMENTE esta shape:
  - `cobrado_vs_por_cobrar`: `[{mes: "2026-09", cobrado, por_cobrar}]` — 6
    meses (el mes actual + 5 previos). cobrado = pagos registrados del mes;
    por_cobrar = cuotas esperadas de ese mes en planes no liquidados.
  - `tasa_morosidad`: f64 0..1 = planes Moroso / planes no liquidados.
  - `flujo_proyectado`: `[{horizonte: 30, monto}, {60, ...}, {90, ...}]` —
    cuotas que vencen en los próximos 30/60/90 días de planes Activo/Moroso.
  - `aging`: `[{bucket: "0-30", monto}, {"31-60"}, {"61-90"}, {"90+"}]` —
    saldo vencido por antigüedad de la cuota (días desde su vencimiento).
  - `top_deudores`: `[{cliente_curp, nombre, saldo}]` — saldo = total a pagar
    (pago_mensual × plazo) − pagos registrados, desc, máx 10 (join con
    `clientes` para el nombre).
  - `distribucion_montos`: `[{bucket: "0-1k", n}, {"1k-5k"}, {"5k+"}]` — n de
    planes por `monto_total`.
- **`dashboard_stats`** (upsert existente): `creditos_activos` = planes
  Activo+Moroso; `proximos_cobros` = cuotas que vencen en 30 días. Sin
  romper el shape actual.
- **WhatsApp plantilla de autenticación**: `WhatsAppOtpSender` deja de mandar
  mensaje libre y envía PLANTILLA (fuera de la ventana de 24 h solo pasan
  plantillas): `template: {name: env WHATSAPP_TEMPLATE (default
  "pymza_otp_verification"), language: {code: env WHATSAPP_TEMPLATE_LANG
  (default "es")}, components: [{type: "body", parameters: [{type: "text",
  text: codigo}]}]}`. Si el envío falla → eprintln y continuar (el flujo de
  dev con mock no se rompe). `WHATSAPP_WABA_ID` NO se usa para enviar (solo
  gestión) — no agregarlo al código sin motivo.
- **TTL index** (O2 del auditor ola 3): crear índice TTL (`expireAfterSeconds:
  0`) sobre `verificaciones.expira_en` (lazy, al construir el cliente DB o al
  primer `solicitar`).
- Deps nuevas: **NINGUNA**. Las gráficas son SVG puro en rsx.

### Gráficas diferidas (sin datos que las soporten hoy — decision log)

De la captura de V: "Ventas a crédito vs contado" (no existe captura de
ventas contado), "Tendencia de recuperación mensual" (necesita ≥2 meses de
pagos reales), "Intereses generados vs pérdidas por impago" (falta marcar
impagos), "Score de salud de cartera" (KPI compuesto, cuando Tier 1-2
maduren), "Predicción de incobrables ML" (ponytail: regresión simple con ≥6
meses de datos; ML real no se justifica hoy), "Costo de oportunidad"
(derivado).

### Mapa de propiedad de archivos

| Archivo/glob | Dueño |
|-----------|-------|
| `backend/Cargo.toml`, `backend/src/**`, `backend/scripts/**`, `docs/API.md`, `.env.example` | executor-1 |
| `frontend/src/**`, `frontend/Dioxus.toml` (solo title/favicon), `frontend/tailwind.css`, `frontend/assets/**` | executor-2 |

Fuera de ambos (nadie toca): `frontend/tailwind.sh`, `frontend/AGENTS.md`,
`frontend/Cargo.toml` (cero deps), `AGENTS.md` (raíz), `README.md`,
`docs/ROADMAP.md`, `docs/INVESTIGACION.md`, `PYMZA.md`, `docker-compose.yml`,
`Dockerfile.*`, `.workflow/**`, `skills/**`, `backend/.env`.

### Tareas

- [x] T1 (executor-1): pagos + estados de plan + resumen para gráficas + plantilla WhatsApp + TTL → brief: `.workflow/briefs/wave4-executor-1.md`
- [x] T2 (executor-2): primitivas SVG + 6 gráficas en dashboard + registrar pago en cartera + favicon/título → brief: `.workflow/briefs/wave4-executor-2.md`

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
# crear un plan de prueba (evaluar + autorizar) y tomar su _id de GET /api/creditos
PLAN_ID=$(curl -s http://127.0.0.1:3000/api/creditos -H "Authorization: Bearer $TOKEN" | jq -r '.creditos[0]._id')
curl -s -X POST http://127.0.0.1:3000/api/creditos/pagos -H "Authorization: Bearer $TOKEN" \
  -H 'content-type: application/json' \
  -d "{\"plan_id\":\"$PLAN_ID\",\"cuota\":1,\"monto\":<pago_mensual del plan>}" | jq .
# duplicado → 400; cuota 2 de un plan de 12 → estado sigue Activo; pagar todas → Liquidado
curl -s http://127.0.0.1:3000/api/creditos/resumen -H "Authorization: Bearer $TOKEN" | jq .resumen.tasa_morosidad
curl -s http://127.0.0.1:3000/api/creditos/resumen -H "Authorization: Bearer TOKEN_INVENTADO" -o /dev/null -w "%{http_code}\n"   # → 401

# 3. Humo UI (navegador, humano): gráficas bajo los KPIs, badge de estado en
#    cartera, registrar pago desde UI, favicon visible en la pestaña, título
#    "PYMZA" en la pestaña.
```

Integrador actualiza los estados de la tabla de olas tras cada paso.

### Audit gate

El auditor corre `.workflow/audit-checklist.md` sobre el árbol integrado y
además verifica (evidencia = salida de comandos):

- Pago duplicado → 400; pago de cuota inexistente → 400; monto ≠ pago_mensual
  → 400; plan ajeno (otro tenant) → 404/403 (curl en vivo).
- Plan con todas las cuotas pagadas → `Liquidado`; plan con cuota vencida sin
  pagar → `Moroso` (en vivo, con fechas manipuladas o plan antiguo).
- `GET /api/creditos/resumen` es tenant-scoped: con 2 tenants, los datos no
  cruzan (en vivo).
- Shape del resumen exacto al contrato (jq por cada campo).
- Plantilla WhatsApp: test del payload builder; sin token el envío falla suave
  (log, no panic); `WHATSAPP_TEMPLATE` default correcto.
- TTL index en `verificaciones` (`mongosh` → `getIndexes()` muestra
  `expireAfterSeconds`).
- Favicon: `frontend/assets/favicon.svg` existe, referenciado en `main.rs`
  (`rel: "icon"`), `Dioxus.toml` title ≠ "frontend".
- Cero deps nuevas (`git diff <base> -- **/Cargo.toml` vacío).
- Nada fuera del mapa de propiedad (`git log --stat` por rama).
- Resultado en `.workflow/audits/wave4.md`.

---

## Decision log

Olas 1–3 (contexto histórico; detalle en `.workflow/audits/wave1.md`,
`wave2.md`, `wave3.md`):

| Fecha | Decisión | Por qué |
|------|----------|-----|
| 2026-08-13 | Tenant key = `correo` de empresa, no ObjectId nuevo | Ya es único y validado; docs existentes se migran con script |
| 2026-08-13 | JWT HS256 (`jsonwebtoken` v9), secret por env, exp 24h | Lo mínimo que funciona; techo: refresh tokens + httpOnly cookies |
| 2026-08-13 | App de cobradores y Tauri fuera de este plan | Productos separados; dependen de que la red esté viva |
| 2026-08-13 | OTP por WhatsApp: proveedor objetivo = WhatsApp Cloud API (Meta) | Llamada directa del backend al proveedor es lo mínimo que funciona |
| 2026-08-17 | VistaPública sin router; auto-login tras registro; default tema dark; `API_BASE` vía `option_env!` | Mínimos que funcionan, techos nombrados |
| 2026-08-28 | Re-segmentación 1: identidad (3) separada de OCR/recibos (4) | Colisiones en main.rs/Cargo.toml/models impedían paralelismo |
| 2026-08-31 | Ola 3 APPROVED. D1: CURPs del seed corregidas a dv reales (05/02). D2: `OtpSender` por `OnceLock` global | Auditor aprobó ambas; techo documentado |
| 2026-08-31 | Observaciones ola 3: O1 hash demo roto (owner V: re-seed), O2 TTL pendiente (→ ola 4), O3 datos de humo en Atlas (owner V) | O2 se resuelve en esta ola |

Ola 4 (nuevas):

| Fecha | Decisión | Por qué |
|------|----------|-----|
| 2026-08-31 | Re-segmentación 2 → 7 olas: ola 4 = pagos + gráficas + favicon (pedido directo de V); 5 = OCR/recibos; 6 = contrato PDF + producción + release gate; 7 = Stripe + ecosistema | Las gráficas de V exigen datos reales de cobranza; registrar pagos es la feature raíz que las desbloquea |
| 2026-08-31 | Las gráficas son SVG puro en rsx (bar/line/donut/hbar), cero librería de charts | Sin deps nuevas; primitivas de 3 tipos cubren las 6 gráficas. Techo: librería si se necesita interactividad avanzada (tooltips/zoom) |
| 2026-08-31 | Estados de plan con ciclo de vida real: Activo → Moroso (cuota vencida sin pagar) → Liquidado (todo pagado) | Hoy "Activo" es fijo; sin esto la morosidad y el aging son mentira |
| 2026-08-31 | Gráficas diferidas: ventas crédito/contado, tendencia de recuperación, intereses vs pérdidas, score de salud, predicción ML, costo de oportunidad | No hay datos reales que las soporten (no se registran ventas contado ni impagos; ML con <6 meses de pagos es decoración) — se activan cuando existan |
| 2026-08-31 | WhatsApp: envío por PLANTILLA de autenticación (nombre por env `WHATSAPP_TEMPLATE`), no mensaje libre | Fuera de la ventana de 24h Meta solo permite plantillas; V creará `pymza_otp_verification` cuando la verificación de Meta Business se apruebe (en curso, SLA 24-48h) |
| 2026-08-31 | Verificación CURP oficial (RENAPO): no existe API pública; vías = convenio RENAPO directo (trámite legal) o proveedores KYC comerciales con API (Verificamex, JAAK, Truora, etc., pago por consulta, hay prueba gratis) | Se integra en la ola 7 con trait `VerificadorCurp` (mismo patrón que `OtpSender`: mock hoy, proveedor cuando V firme); hoy la redundancia es dígito verificador + OTP + OCR INE (ola 5) |