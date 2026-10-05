# Plan: PYMZA — Perfilación de Crédito y Cobranza para PYMES

> Single source of truth del trabajo. Commiteado, sobrevive cualquier sesión.
> SOLO la siguiente ola está detallada (plan rodante). Si una sesión muere, la
> nueva instancia reanuda desde este archivo — nunca desde memoria.

## Goal

PYMZA en producción (Railway) y usándose **gratis por empresas reales** en fase
de testing, contra MongoDB Atlas: registro, login JWT, alta/búsqueda de clientes,
evaluar/autorizar créditos multi-tenant, cartera, pagos, contrato PDF y
dashboard. Verificado en vivo 2026-09-29: frontend
`triumphant-commitment-frontend.up.railway.app`, backend
`pymza-production-3a22.up.railway.app` (401 sin token; login demo `demo@pymza.mx`
→ 200 con token).

Ciclo actual (olas 7+): convertir ese MVP en el producto que las empresas
confíen a diario — cobranza real (abonos, tasas, saldos), tablero que no mienta,
control por empleado, avisos de novedades — **sin tocar en duro la DB real**:
todo cambio de esquema es aditivo y con defaults, ninguna migración destructiva.

Lo que NO está en este plan (explícitamente fuera): la app de cobradores
("uber de cobranza") es un producto móvil separado; Tauri/escritorio: web
primero. Facturación CFDI: se evalúa como integración con un PAC en la ola 12,
nunca como ERP propio.

## Stack & constraints

| Capa | Tech |
|---|---|
| Frontend | Dioxus 0.7.9 (pin `=0.7.9`) Rust → WASM + Tailwind v4. `frontend/AGENTS.md` es la referencia API obligatoria. `API_BASE` configurable por build (ola 2) |
| Backend | Axum 0.6 / Tokio. Modularizado: `routes/`, `models/`, `auth.rs`, `otp.rs`, `ocr.rs` |
| DB | MongoDB Atlas (real) vía `MONGODB_URI`. Colecciones: `empresas`, `clientes`, `planes_pago`, `dashboard_stats`, `verificaciones`, `pagos`, `recibos` |
| Infra | Docker Compose + Dockerfiles (tesseract en imagen backend). **Producción viva en Railway** (backend + frontend + Atlas); V despliega cada ola con `docs/DEPLOY.md` |

Constraints:
- Secretos nunca al repo: `MONGODB_URI`, `JWT_SECRET`, credenciales de proveedores (WhatsApp, Stripe, CdC, KYC) solo en `.env` local / variables de Railway.
- `Cargo.lock` gitignored — normal al añadir deps.
- NixOS: `dx` no compila Tailwind; el CSS compilado está commiteado. Regenerar con `frontend/tailwind.sh` si una ola cambia clases.
- Sin CI. Verificación: `cargo test` + `cargo check --target wasm32-unknown-unknown`.
- OCR: binario `tesseract` (Docker: `tesseract-ocr` + `tesseract-ocr-spa`).
- **Release gate (toda ola que se despliegue a producción): `skills/security-audit` con cero CRITICAL/HIGH (o excepciones documentadas con owner) ANTES de desplegar a Railway** — hay empresas reales usándola y la DB es real.
- Demo real en Atlas: `demo@pymza.mx` / `demo1234`.

## Waves

| Ola | Foco | Estado |
|-----|------|--------|
| 1 | Cimientos: JWT real + multi-tenant + frontend modularizado | [x] auditada 2026-08-17 |
| 2 | Portal público: landing, registro/login, tema claro/oscuro, `API_BASE` configurable | [x] auditada 2026-08-28 |
| 3 | Identidad verificable: CURP dv, correo, OTP teléfono (WhatsApp/mock) | [x] auditada 2026-08-31 |
| 4 | Cartera viva: pagos + estados de plan + gráficas SVG + favicon | [x] auditada 2026-09-04 |
| 5 | KYC/OCR real (tesseract) + score alternativo por recibos | [x] auditada 2026-09-05 (APPROVED WITH EXCEPTIONS: E1 413→ola 6, E2 fixture→ola 6) |
| 6 | Contrato PDF + Producción: CORS productivo, body limit, rate limiting, Dockerfiles Railway, security audit (release gate) | [x] auditada 2026-09-06 — **REJECTED** (F1/F2 HIGH + S1 Dockerfile) → hotfix en ola 6-fix |
| 6-fix | Hotfix release gate: F1/F2 (validación plazo/monto), S1 (Dockerfile.backend), S2 (CSS) | [x] auditada 2026-09-06 (APPROVED WITH EXCEPTIONS: A6-1 LOW → ola 7) — **release gate CERRADO**: F1/F2 corregidos en vivo, ambas imágenes Docker construyen (tesseract+spa, no-root), índice único verificado. V despliega con `docs/DEPLOY.md` |
| 7 | Cobranza real y cartera usable: abonos, tasas 1 mes 7% escalonado, saldo/estado por dinero, contrato con abonos y liquidación, buscador+filtros+dos tablas en cartera, nombre del cliente | [x] integrada 2026-10-04 (pendiente auditoría) |
| 8 | Tablero que dice la verdad: KPIs cobrado/por cobrar/capital, morosidad honesta, filtros por periodo, gráficas corregidas + campanita de novedades ("what's new") | [ ] |
| 9 | Confianza y control: sub-usuarios por empresa (roles + auditoría de quién hizo qué), aval en alta de cliente, catálogo de productos con ID | [ ] |
| 10 | Dinero y verificación: Stripe (suscripción), validación de correo de empresa, Verificamex (CURP/teléfono), score real (adiós al 550 fijo) | [ ] |
| 11 | Documentos y firma: firma digital (pad), contrato firmado por correo, documentos del cliente accesibles a la red con compresión automática | [ ] |
| 12 | Ecosistema: frontends inversionistas/soporte, buró CdC/FICO, open banking, procesar pagos de deudores vía PYMZA, CFDI/PAC | [ ] |

> Estados: planificada → en vuelo → integrada → auditada → hecha.
> Actualizar después de cada paso, quien lo ejecute.

---

## Ola 7 (actual): cobranza real y cartera usable

Contexto: PYMZA ya está en producción usándose por empresas reales. La
retroalimentación de una de ellas (`PYMZA.md`, sección "Observaciones dadas por
parte de las empresas que les urge implementar", Sep 28 2026) marca lo que hoy
hace que la cobranza mienta o estorbe:

1. **Abonos**: hoy `POST /api/creditos/pagos` exige el monto exacto de la cuota y
   marca la cuota como pagada; no existe el abono parcial ("cada semana le van
   abonando, pero no marcar la cuota hasta saldar").
2. **Plan a 1 mes con 7%** y tasas escalonadas coherentes (hoy el mínimo es 3
   meses y la tabla no admite un 1 mes).
3. **Saldo y estado por dinero**: hoy `estado` depende de cuotas marcadas y solo
   se recalcula al registrar un pago (queda rancio en lectura); "Liquidado" no
   refleja abonos.
4. **Contrato** que muestre abonos/pagos y selle el plan cuando queda liquidado.
5. **Cartera usable**: buscador por nombre/ID/CURP, filtros, tabla de activos
   separada de la de liquidados, y el nombre del cliente (hoy la cartera solo
   muestra CURP).

División: **executor-1 backend, executor-2 frontend**. El despliegue a Railway lo
sigue ejecutando V (con `docs/DEPLOY.md`) después de la auditoría.

### Contrato API ola 7 (ambos executors implementan contra ESTO)

**Deuda y saldo (semántica canónica: backend, PDF y frontend usan esta):**

- `deuda = pago_mensual * plazo_meses` (el total con interés que ya usa `top_deudores`).
- `cobrado = suma de TODOS los pagos y abonos del plan`.
- `saldo = max(0, deuda - cobrado)` (redondeado a 2 decimales).
- `cuotas_cubiertas = min(plazo, floor(cobrado / pago_mensual))`.
- `n_vencidas` = cuotas con `fecha_vencimiento(plan.fecha, n) < hoy`;
  `cuotas_vencidas = min(plazo, max(0, n_vencidas - cuotas_cubiertas))`.
- `estado`: `Liquidado` si `saldo <= 0.01`; si no, `Moroso` si
  `cuotas_vencidas > 0`; si no, `Activo`.
  Puro y testeado: la función v2 recibe `cobrado` en lugar de la lista de cuotas.
- **Nada de migración**: `Pago` gana `tipo` (`"cuota"` | `"abono"`) con
  `#[serde(default)]` → los pagos viejos sin el campo leen como `"cuota"`. Los
  abonos se guardan con `cuota = 0`. `PlanPago` NO cambia de esquema.

**Endpoints:**

- `POST /api/creditos/abonos` (protegido): body `{plan_id, monto, nota?}`.
  Validaciones en orden: plan del tenant → 404 (mismo lookup único que
  `registrar_pago`); `monto` finito y `> 0` → 400; plan ya `Liquidado` → 400;
  `monto > saldo + 0.01` → 400 ("el abono excede el saldo pendiente"). Inserta
  `Pago{tipo:"abono", cuota:0}` y devuelve el plan actualizado con la shape de
  `registrar_pago` (más `saldo`/`cobrado`).
- `POST /api/creditos/pagos`: path y body intactos (pago de cuota exacta).
- `GET /api/creditos`: cada plan añade `nombre` (join con `clientes` por `$in` de
  los CURPs del tenant — UNA query, no N), `cobrado`, `saldo`; `cuotas_pagadas`
  con la semántica nueva (`cuotas_cubiertas`) y `estado` **recalculado en
  lectura** (nunca el persistido).
- `GET /api/creditos/:plan_id/contrato`: mismo path. El PDF incluye una sección de
  **abonos/pagos registrados** (fecha, tipo, monto), el `cobrado`/`saldo` al
  momento de emitirse y el sello `LIQUIDADO — FINIQUITO` cuando `saldo <= 0.01`.
- `GET /api/dashboard`: sin cambios de shape en esta ola (el tablero es ola 8);
  pero `upsert_dashboard_stats` debe usar el estado recalculado.

**Tasas (APROBADAS por V 2026-09-29):**
`tasa_por_plazo` pasa a `{1: 0.07, 3: 0.09, 6: 0.12, 9: 0.15, 12: 0.18}` y
`validar_plazo_y_monto` acepta SOLO esos plazos (400 con mensaje claro si no).
Los planes ya guardados conservan su `tasa_interes` y su `pago_mensual` (no se
recalculan).

**Índice (cierra F6 del ledger):** índice único parcial en `pagos` sobre
`{plan_id, cuota}` con `partialFilterExpression: {cuota: {$gt: 0}}` — cierra la
carrera de doble pago de cuota sin bloquear los abonos (`cuota: 0`). Se crea en
`db.rs::connect` igual que el de `empresas.correo` (idempotente, no fatal).

**Frontend:**

- `plan_modal.rs`: opción "1 mes — Tasa 7%" + tasas nuevas en el select.
- `cartera.rs`:
  - Buscador único por nombre / CURP / `_id` de plan + filtro por estado y
    producto (filtrado en memoria: los planes de una PYME son pocos miles).
  - Dos tablas: **Activos** (Activo + Moroso) y **Liquidados / inactivos** debajo,
    cada una con su búsqueda/filtro y orden por columna (fecha, monto, saldo).
  - Columnas nuevas: nombre del cliente, saldo, "X/Y cubiertas".
  - Botón **Registrar abono** por plan (independiente de "Registrar pago"): form
    inline con monto (default = saldo, editable) y nota opcional.
- `api.rs`: `registrar_abono(plan_id, monto, nota, token)` + parsing de
  `saldo`/`cobrado`.
- Regenerar `frontend/assets/tailwind.css` con `./tailwind.sh` si hay clases nuevas.

### Mapa de propiedad de archivos (ola 7)

| Archivo/glob | Dueño |
|-----------|-------|
| `backend/src/routes/credito.rs`, `backend/src/models/credito.rs`, `backend/src/pdf.rs`, `backend/src/db.rs`, `backend/src/main.rs` (SOLO alta de la ruta `/api/creditos/abonos`), `docs/API.md`, `backend/scripts/**` | executor-1 |
| `frontend/src/components/cartera.rs`, `frontend/src/components/plan_modal.rs`, `frontend/src/api.rs`, `frontend/assets/tailwind.css`, `frontend/tailwind.css` | executor-2 |

Fuera de ambos (nadie toca): `frontend/src/main.rs`,
`frontend/src/components/dashboard.rs`, `frontend/src/components/charts.rs`,
`frontend/src/components/alta_cliente.rs`, `frontend/src/components/sidebar.rs`,
`backend/src/routes/cliente.rs`, `backend/src/models/cliente.rs`, `.env*`,
`.workflow/**`, `skills/**`, `PYMZA.md`, `AGENTS.md`, `docs/DEPLOY.md`,
`docs/ROADMAP.md`, `docs/INVESTIGACION.md`, el resto de servicios de
`docker-compose.yml`, `Dockerfile.*`.

### Tareas

- [x] T1 (executor-1): abonos + tasas 1 mes + saldo/estado por dinero + contrato con abonos y liquidación + nombre en cartera + índice parcial → brief `.workflow/briefs/wave7-executor-1.md`
- [x] T2 (executor-2): cartera buscador/filtros/dos tablas + botón de abono + plan 1 mes en el modal → brief `.workflow/briefs/wave7-executor-2.md`

### Arranque de la ola 7 (launch kit)

Un executor por brief, en su propio worktree (branch isolation obligatoria). Cada
uno lee SOLO su brief y el plan; el contexto del planner no se hereda.

```bash
# Desde main (limpio, con los briefs commiteados):
git worktree add ../pymza-w7-e1 -b wave7-executor-1 main
git worktree add ../pymza-w7-e2 -b wave7-executor-2 main
# En cada worktree: leer .workflow/briefs/wave7-executor-K.md, implementar,
# verificar, commit + `git push origin wave7-executor-K`. Nunca tocar main.
# Al terminar: git worktree remove ../pymza-w7-e1  (lo hace el integrador)
```

### Plan de integración (ola 7)

Merges en orden: **executor-1 (backend) → executor-2 (frontend)**.

```bash
# 1. Build + tests sobre el árbol integrado
cd backend && cargo build && cargo test
cd frontend && cargo check --target wasm32-unknown-unknown && cargo test && ./tailwind.sh

# 2. Humo local (mongod local + seed demo). Sin tocar Atlas.
cd backend && cargo run
TOKEN=$(curl -s -X POST http://127.0.0.1:3000/api/login -H 'content-type: application/json' -d '{"correo":"demo@pymza.mx","password":"demo1234"}' | jq -r .token)
# plan 1 mes → tasa 0.07
curl -s http://127.0.0.1:3000/api/creditos/evaluar -H "Authorization: Bearer $TOKEN" -H 'content-type: application/json' \
  -d '{"curp":"<curp-seed>","monto":1000,"plazo_meses":1}' | jq '.tasa_interes, .pago_mensual'
# abono parcial: saldo baja y la cuota NO se marca pagada
# abono > saldo → 400 · plan ajeno → 404 · plan liquidado → 400 · sin token → 401
# contrato de plan con abonos: %PDF y sello LIQUIDADO si saldo 0
```

Integrador actualiza los estados de la tabla de olas tras cada paso.

### Audit gate (ola 7)

- **Release gate: `skills/security-audit` sobre el árbol integrado; cero
  CRITICAL/HIGH (o excepciones documentadas con owner)** — la ola toca registro
  de dinero contra una DB real.
- `cargo test` backend+frontend en verde en el árbol integrado; cero deps nuevas
  (`git diff Cargo.toml`).
- Regresión de pagos viejos: un `Pago` sin campo `tipo` cuenta como cuota y el
  resultado de saldo/estado es idéntico al de hoy.
- Semántica: deuda D, abono A → `saldo = D−A`; plan cubierto por abonos queda
  `Liquidado`; cuota vencida sin dinero → `Moroso`.
- Aislamiento: abono/contrato sobre plan ajeno → 404; sin token → 401.
- Entradas: `monto <= 0`, `NaN/Inf`, mayor al saldo → 400; plan liquidado → 400.
- Índice parcial: existe en `pagos`, rechaza cuota duplicada, permite varios abonos.
- Contrato: header `%PDF`, sección de abonos, sello de liquidado, y la tabla suma
  `pago_mensual * plazo_meses`.
- CSS regenerado si hubo clases nuevas.
- Humo UI en navegador (owner V): buscador, filtros, dos tablas, abono, contrato.

---

## Olas 8-12 (foco, sin detallar — plan rodante)

- **Ola 8 — Tablero que dice la verdad + novedades.** KPIs honestos (cobrado real,
  por cobrar neto, capital colocado, morosidad sobre dinero y no sobre planes),
  filtros por periodo (semana / mes / bimestre / trimestre / semestre) y
  corrección de las 6 gráficas del `resumen`. Incluye la **campanita de novedades**
  ("what's new"): versión de la app + `GET /api/novedades` que devuelve la última
  versión y el changelog; si la versión compilada del WASM es menor que la del
  servidor, el usuario ve "hay una actualización, recarga" y el bell lista los
  cambios. Techo conocido: solo se anuncia a quien ya trae esta build (o superior).
- **Ola 9 — Confianza y control interno.** Sub-usuarios por empresa con rol fijo
  (subadmin / cajero / vendedor / cobrador) y credenciales propias, **auditoría**
  de quién hizo cada acción, **aval** en el alta de cliente, **catálogo de
  productos** con ID, y **editar/cancelar créditos** de la propia cartera por el
  admin. Diseño en §"Identidad, roles y edición de cartera (ola 9)".
- **Ola 10 — Dinero y verificación real.** Suscripción con Stripe (plan por
  empresa, **solo cuando el producto esté pulido**: hoy sigue gratis), validación
  del correo de empresa, Verificamex para CURP/teléfono (deja de ser heurística) y
  **score real** de PYMZA (sustituir el 550 fijo por fórmula con historial de la
  red + recibos). CdC/FICO se integran después (ver §"Score: red primero,
  buró después").
- **Ola 11 — Documentos y firma.** Firma digital en pad (pantalla/lápiz) incrustada
  en el contrato, envío del contrato firmado por correo, y documentos del cliente
  (INE, recibos, aval) comprimidos automáticamente (<2MB) y accesibles a la red
  PYMZA desde el perfil del cliente.
- **Ola 12 — Ecosistema.** Frontend de inversionistas (métricas/consumo), frontend
  de servicio técnico de PYMZA, buró Círculo de Crédito/FICO (sandbox→producción),
  open banking, procesar pagos de deudores a través de PYMZA para liquidar en
  tiempo real, y CFDI vía PAC si una empresa lo exige.

---

## Identidad, roles y edición de cartera (diseño para ola 9)

Aprobado por V. Hoy cada empresa tiene UNA cuenta (`empresas`, tenant = correo);
todos los que conocen la contraseña hacen de todo y no se sabe quién. El objetivo
no es solo seguridad: es que **la responsabilidad tenga nombre**.

**Modelo propuesto (aditivo, sin migración):**

- Nueva colección `usuarios`: `{_id, empresa (correo), nombre, usuario (handle de
  login), password_hash (argon2id, el mismo de `auth.rs`), rol, activo,
  creado_en, creado_por}`. Índice único `(empresa, usuario)` (idempotente en
  `db.rs`, como el de `empresas.correo`).
- **Admin raíz = la empresa actual**: `POST /api/login` sigue aceptando
  `{correo, password}` contra `empresas` → ese token es `rol: "admin"`. Si no
  coincide, intenta contra `usuarios` por `usuario` (el admin puede usar el correo
  o un handle corto). JWT nuevo gana claims `uid`, `rol`, `nombre`; `sub` sigue
  siendo el correo de la empresa (tenant intacto). El extractor `EmpresaSession`
  expone `correo`, `usuario_id`, `rol`, `nombre`.
- El admin crea usuarios por **nombre** y elige un **rol predeterminado**; no hay
  constructor de permisos (menos trabajo y menos errores). Desactivar (`activo:
  false`) en vez de borrar, para no perder la autoría de lo que hizo.

**Roles fijos (propuesta, matriz aplicada en el BACKEND, no solo en la UI):**

| Acción | admin | subadmin | vendedor | cajero | cobrador |
|---|---|---|---|---|---|
| Ver cartera y dashboard | ✅ | ✅ | ✅ | ✅ | ✅ |
| Alta de cliente (red, por CURP) | ✅ | ✅ | ✅ | ✅ | ❌ |
| Evaluar / autorizar crédito | ✅ | ✅ | ✅ | ❌ | ❌ |
| Registrar pago / abono | ✅ | ✅ | ✅ | ✅ | ✅ |
| Reportar alerta de morosidad | ✅ | ✅ | ✅ | ✅ | ✅ |
| **Editar / cancelar crédito** | ✅ | ✅ | ❌ | ❌ | ❌ |
| Gestionar usuarios y roles | ✅ | ✅ | ❌ | ❌ | ❌ |
| Ver auditoría del tenant | ✅ | ✅ | ❌ | ❌ | ❌ |

> `subadmin` existe para que el dueño delegue la operación sin dar la cuenta
> raíz. Si a V le parece de más, se cae y quedan 4 roles.

**Auditoría (quién hizo qué):**

- Campos de autoría (aditivos, los docs viejos leen `None` = "legacy"):
  `planes_pago.autorizado_por`, `pagos.registrado_por`,
  `clientes.creado_por`, y `cancelado_por`/`cancelado_motivo` al cancelar.
- Colección `eventos`: `{empresa, usuario_id, usuario_nombre, rol, accion,
  entidad, entidad_id, fecha, detalle}` para el panel "Actividad" del admin.
  Texto libre acotado, nunca datos de la red de otras empresas.

**Editar / cancelar crédito (regla contable, aprobada por V):**

- **Sin pagos registrados**: el admin puede editar (producto, monto, plazo, tasa
  → se regenera el plan) o borrar. Nada que corromper.
- **Con pagos registrados**: SOLO cancelar (soft: `estado = "Cancelado"` +
  motivo + quién). No se reescribe dinero nunca. El plan cancelado baja a la
  tabla de inactivos y sale de aging/tablero. Para corregir montos ya con pagos:
  cancelar y autorizar un plan nuevo.
- **Perfiles de cliente**: NUNCA los edita la empresa; el perfil es de la red y
  solo PYMZA lo toca (frontend de soporte, ola 12). La empresa solo administra su
  cartera.

---

## Score: red primero, buró después (aclaración CdC vs FICO)

- **Círculo de Crédito** (y Buró de Crédito) son las **SIC**: las bases de datos
  que guardan el historial crediticio real. Eso es lo que se contrata como
  otorgante (por consulta, bajo contrato).
- **FICO** no es una fuente: es un **modelo de score propietario que se vende a
  través de las SIC**. No se contrata con FICO; se consume la API del buró y, si
  el producto lo incluye, viene el score FICO. Su fórmula es cerrada: no se
  replica; a lo sumo se imita el enfoque (probabilidad de incumplimiento).
- **PYMZA (aprobado):** construir el score propio primero con la red (historial
  interno de pagos/abonos y morosidad) + recibos de servicios (heurística ya
  existente, se formaliza). Cuando haya volumen y entidad jurídica, integrar un
  buró (sandbox→producción) y opcionalmente su score FICO. Detalle de proveedores
  y costos en `docs/INVESTIGACION.md`.

---

## Nota competitiva: Microsip (competidor directo hoy)

Las empresas usuarias ya usan [Microsip](https://www.microsip.com/) — un **ERP**
mexicano con 40 años, ~100k clientes y 350 partners: SAT/CFDI 4.0, contabilidad,
bancos, nómina, inventarios, ventas, punto de venta, cuentas por pagar/cobrar,
Sync-E, administrador de sucursales, CEO móvil, portal de suscripción y soporte
humano. No competimos de frente: **PYMZA es la red de crédito y cobranza, no el
ERP de la empresa.**

Paridad que sí bloquea adopción (por eso entra en las olas 8-12, no en todas):

| Lo que una PYME ya espera de Microsip | Respuesta PYMZA | Ola |
|---|---|---|
| Roles y permisos por empleado; saber quién hizo cada movimiento | Sub-usuarios con rol + auditoría | 9 |
| Cuentas por cobrar: estado de cuenta por cliente, antigüedad de saldos, abonos parciales | Ola 7 (abonos/saldo) + ola 8 (aging honesto) + estado de cuenta por cliente en ola 9 | 7-9 |
| Documentos imprimibles/enviables (estados de cuenta, recibos, contratos) | Contrato PDF ya existe; se actualiza en ola 7 y se firma/emaila en ola 11 | 7, 11 |
| Reportes/KPIs para decidir (CEO móvil) | Tablero honesto + filtros de periodo | 8 |
| Suscripción y portal de pago | Stripe | 10 |
| Soporte y comunidad | Frontend de servicio técnico (SLA de atención) | 12 |
| Facturación CFDI 4.0 | Integración con PAC (no ERP) | 12 |
| Punto de venta / Sync-E / en ruta / inventarios | **Fuera** de PYMZA | — |

Diferenciador que Microsip no tiene ni tendrá pronto: **red colaborativa de
alerta temprana, perfil global reutilizable entre comercios, score para clientes
sin buró (recibos de servicios) y onboarding por CURP en segundos.** El mensaje de
venta: "Microsip administra tu empresa; PYMZA te protege de tus clientes."

---

## Ola 6 (histórica): contrato PDF + producción (release gate)

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

## Ola 6-fix (mini-ola): hotfix del release gate

Contexto: la ola 6 quedó **REJECTED** (`.workflow/audits/wave6.md`): F1/F2 HIGH
confirmados en vivo (DoS con `plazo_meses` sin validar; plan envenenado
persistido), `Dockerfile.backend` no construye (S1) y CSS sin regenerar (S2).
La auditoría ya validó las recetas de fix — esta ola solo las aplica. Todo lo
demás del gate está en verde (tenant isolation del PDF PASA, CORS, E1/E2
cerradas, rate limit OK, 68+40 tests).

**Detalles del fix ya validados por el auditor (no re-inventar):**
- **F1/F2** (HIGH): `evaluar` con `plazo_meses` gigante → `collect()` ~86 GB →
  OOM (RSS 30 GB medido); `autorizar` persiste el plan envenenado → cartera
  congelada, contrato-PDF = OOM persistente. Fix: validar `plazo_meses ∈ 3..=12`
  y `monto > 0` finito en `evaluar_credito` Y `autorizar_credito` → 400 con el
  mensaje del contrato. ~15 líneas + tests (payloads de prueba: `plazo_meses:
  1000000`, `monto: -1`, `monto: 1e400` — este último ya devuelve 400 hoy).
- **S1** (bloqueante de Railway): `Dockerfile.backend` tiene 2 bugs:
  `COPY backend/Cargo.toml backend/Cargo.toml` (destino equivocado → "could
  not find Cargo.toml in /build") y `FROM rust:1.83-bookworm` (ya no compila
  `time-core-0.1.9`, edition2024). Fix validado EN VIVO por el auditor:
  `COPY backend/Cargo.toml ./Cargo.toml` + `COPY backend/src ./src` +
  `FROM rust:1.97-bookworm AS builder` → build OK, imagen con
  `tesseract 5.3.0 + spa`. Sin esto Railway no despliega.
- **S2** (menor): el botón "Descargar contrato" usa 4 clases Tailwind que no
  están en `assets/tailwind.css` (`hover:bg-blue-700`, `py-1.5`,
  `hover:bg-slate-300`, `dark:bg-slate-700`). Fix: `cd frontend &&
  ./tailwind.sh` + commit del CSS.
- **T4 OPCIONAL (solo si V lo aprueba) — F5**: carrera en `alta_empresa` sin
  índice único en `empresas.correo` → dos registros simultáneos comparten
  tenant key (cross-tenant total para ese correo). Fix barato: índice único en
  la inicialización del pool (`db.rs`). Recomendado antes de que el cliente
  real empiece a probar esta semana.

### Mapa de propiedad (6-fix)

| Archivo/glob | Dueño |
|-----------|-------|
| `backend/src/routes/credito.rs` (F1/F2 + tests), `Dockerfile.backend` (S1), `backend/src/db.rs` (T4 opcional) | executor único |
| `frontend/assets/tailwind.css` (S2, regenerado — nunca a mano) | executor único |

Un solo executor: los tres fixes son ~20 líneas en total y la paralelización
no compra nada. Fuera de alcance: TODO lo demás (sin refactor, sin F3/F4/F6-
F8 — quedan en el ledger para olas 7+).

### Tareas

- [x] T1 (executor): F1+F2 validación plazo/monto en evaluar+autorizar → 400 + tests (commit `a32b096`)
- [x] T2 (executor): S1 Dockerfile.backend (COPY + rust:1.97) (commit `ab551d0`)
- [x] T3 (executor): S2 regenerar CSS compilado — **falso positivo**: las 4 clases ya estaban (forma escapada `hover\:bg-blue-700` etc.); regeneración byte-idéntica (hash `15cdb136`). El grep del auditor ola 6 no escapaba los selectores de Tailwind v4
- [x] T4 (executor, APROBADO por V): F5 índice único `empresas.correo` (commit `31487bb`)

### Plan de integración (6-fix)

Un solo branch → integrador mergea `wave6fix-executor-1` → `main` y corre:

```bash
cd backend && cargo build && cargo test          # 68+passed, 0 failed
cd frontend && cargo check --target wasm32-unknown-unknown && cargo test
docker compose build                              # AMBOS servicios construyen (S1 verificado)
docker run --rm $(docker compose ps -q backend) tesseract --version
```

### Re-auditoría puntual (NO repetir el gate completo)

El auditor solo verifica, en `main` integrado: (1) `cargo test` backend+frontend;
(2) los 3 payloads de F1/F2 → **400** contra el backend corriendo; (3)
`docker compose build` OK + `tesseract --version` dentro de la imagen. Veredicto
al final de `.workflow/audits/wave6.md` (sección "Re-auditoría"). Si APPROVED →
V despliega con `docs/DEPLOY.md`.

### Ledger post-despliegue (olas 7+, del security-audit)

F3 (OTP sin comparar teléfono — MEDIUM), F4 (bucket rate-limiter global en
Railway — MEDIUM, requiere testing en Railway), F6 (TOCTOU `registrar_pago` —
MEDIUM), F7 (enumeración de correos — LOW), F8 (carrera tope de recibos — LOW),
hardening de `mongo:latest` del compose para local (kernel ≥6.19).

### Micro-tarea fuera de ola: verificación de dominio de Meta (destrabar OTP)

Meta pide `<meta name="facebook-domain-verification">` en el `<head>` del sitio.
El shell HTML NO existe como archivo (dx 0.7.9 lo genera desde `Dioxus.toml`;
salida en `target/dx/frontend/release/web/public/index.html`), así que la vía
dx-nativa es crear `frontend/index.html` como template: dx lo usa verbatim y
inyecta automáticamente el `<script>` del WASM. Punto de montaje exacto
requerido: `<div id="main"></div>`. Fallback si el template rompe el build de
dx 0.7.9: un `RUN sed` en `Dockerfile.frontend` justo tras el `dx build`.
Verify del executor: `cd frontend && dx build --release
--debug-symbols=false && grep -c facebook-domain-verification
target/dx/frontend/release/web/public/index.html` → 1.

En Meta: registrar el dominio **completo** del frontend
(`triumphant-commitment-frontend.up.railway.app`), nunca `up.railway.app`
(domain compartido de Railway, no verificable por nosotros). El content de la
etiqueta es público (se sirve en el HTML visible) — commitearlo es seguro.

**RESULTADO (2026-09-07): Micro-tarea completada y dominio VERIFICADO por Meta.**
El executor usó el fallback sed (commit `98b1041` en `Dockerfile.frontend`,
merge `ea3efb0`): no pudo verificar el template nativo en LOCAL porque la
toolchain del host no puede correr `dx build` con este repo (dx instalado en
host = 0.7.10, que rechaza el pin `dioxus =0.7.9`; wasm-bindgen-cli host 0.2.126
vs lock 0.2.128). Verificó en su lugar con el build real de Docker
(dx 0.7.9 instalado dentro del contenedor): exit 0, etiqueta ×1, sin
inyecciones duplicadas, title/div#main intactos — dirección de V confirmó el
estado en vivo (curl: etiqueta presente; Meta: dominio verificado).

**Nota para ola 7 (mecanismo de head-tags):** el único lugar donde corre
dx 0.7.9 es el contenedor Docker. Un `frontend/index.html` nativo quizá funcione
allí, pero quedó sin explorar (peligro: sin verificación local). Próxima vez que
haya que editar el `<head>` (OG tags, favicon, más verificaciones): intentar el
template nativo con verify DENTRO del contenedor (docker build + grep en la
salida) antes del sed; sed solo como plan B. Si el negocio cambia de nombre
(IMPI), migrar el dominio de Meta es tocar una línea del template + nueva
propiedad en Meta — la base de código no conoce la marca.

Pendiente a Meta (bloquea OTP de ola 7): la revisión del app request de
WhatsApp Cloud API sigue en revisión — esperando a Meta.

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
| 2026-09-06 | Ola 6 REJECTED → mini-ola 6-fix (un executor, ~20 líneas): F1/F2 validación plazo/monto, S1 Dockerfile, S2 CSS; T4 F5 índice único opcional si V aprueba | Los fixes ya validados en vivo por el auditor; la paralelización no compra nada. Ledger F3-F8 → olas 7+ |
| 2026-09-06 | S2 = falso positivo: el grep del auditor no escapaba los selectores de Tailwind v4 (`hover\:bg-blue-700`); regeneración del CSS byte-idéntica (hash `15cdb136`) | Lección: los greps sobre CSS compilado de Tailwind deben escapar `:` y `.`; verificación del executor con `git hash-object` = blob en main |
| 2026-09-06 | Ola 6 REJECTED (release gate): F1/F2 HIGH (`plazo_meses` sin validar → OOM ~86GB en evaluar / plan envenenado que congela cartera), Dockerfile.backend no construye (COPY a subdir + rust:1.83 < edition2024), CSS de cartera sin regenerar. Fixs de 3 piezas + re-auditoría puntual; tenant PDF OK, resto del gate en verde | Auditoría en fresco con evidencia en vivo: el humo Docker pendiente era la única red que quedaba y sí atrapó los 2 bugs de build; los límites de entrada de evaluar/autorizar eran un hueco lógico que ningún test cubría |
| 2026-09-07 | Meta domain verification vía template `frontend/index.html` (dx-nativo; fallback sed en Dockerfile) — micro-tarea single-executor fuera de ola, para destrabar OTP | El shell HTML no existe en repo; el template es el mecanismo oficial de dx. Un archivo, un commit — paralelizar no compra nada |
| 2026-09-07 | **Producción viva y verificada**: login demo OK en vivo (200+token), CORS OK (`ALLOWED_ORIGINS` manual de V), WASM con `API_BASE=https://pymza-production-3a22…` compilado. Único fix: variable `API_BASE` del servicio frontend de Railway (estaba vacía → build con fallback `127.0.0.1:3000`). La DB nunca fue el problema; Railway NO auto-conecta servicios | `API_BASE` es compile-time (`option_env!`): variable vacía = build inútil, y cambiarla exige re-build. Lección en DEPLOY.md (`e041843`): en Railway es una Variable del servicio, no "build args"; la UI no tiene esa sección |

Olas 7+ (planificación 2026-09-29):

| Fecha | Decisión | Por qué |
|------|----------|-----|
| 2026-09-29 | Estado real verificado en vivo por el planner: frontend `triumphant-commitment-frontend.up.railway.app`, backend `pymza-production-3a22.up.railway.app` (`/api/dashboard` sin token → 401; login demo → 200) | El plan parte del estado real, no de memoria; la DB es real y hay que cuidarla |
| 2026-09-29 | **Re-segmentación olas 7-12**: 7 cobranza real (abonos/tasas/saldo/contrato/cartera), 8 tablero+novedades, 9 roles/aval/productos, 10 dinero/verificación/score, 11 documentos/firma, 12 ecosistema | La retro de empresas reales (`PYMZA.md` Sep 28) prioriza confianza y uso diario; Stripe/buró (ola 7 vieja) pueden esperar a que el uso diario no tenga fricción |
| 2026-09-29 | Abonos = `Pago.tipo` (`"abono"`, `cuota: 0`) y saldo por **dinero** (`deuda = pago_mensual*plazo`, `saldo = deuda − cobrado`); `estado` se recalcula en lectura | Aditivo: los pagos viejos sin `tipo` leen como cuota y el resultado es idéntico; el estado deja de quedar rancio (hoy solo se recalcula al registrar un pago) |
| 2026-09-29 | Índice único **parcial** en `pagos{plan_id,cuota}` con `cuota > 0` — cierra F6 del ledger sin bloquear abonos | Solo la cuota duplicada es error contable; los abonos se acumulan legítimamente |
| 2026-09-29 | Tasas 1/3/6/9/12 = 7/9/12/15/18% — **PENDIENTE OK de V** | Petición textual de la empresa (1 mes al 7%); el resto queda escalonado monotónico. Los planes guardados no se recalculan |
| 2026-09-29 | La "campanita"/what's new (petición de V) se implementa en ola 8 con `GET /api/novedades` (versión+changelog) y comparación contra la versión compilada; sin websockets ni push | Lo más corto que funciona: poll barato + modal de novedades. Techo: solo anuncia a builds que ya traen la feature |
| 2026-09-29 | Microsip no se copia como ERP: paridad solo donde bloquea adopción (roles, CxC/abonos, documentos, KPIs, suscripción); moat = red de alerta + score alternativo + onboarding por CURP | Competir como ERP sería suicidio de alcance; el diferencial ya está en el producto |
| 2026-09-29 | Elevator Pitch de `PYMZA.md` reescrito con la visión completa (red, informalidad, score alternativo, diferenciación vs Microsip, modelo SaaS) | Toda la idea estaba regada en la nota; el pitch era una sola línea |

Aprobaciones de V (2026-09-29):

| Fecha | Decisión | Por qué |
|------|----------|-----|
| 2026-09-29 | **APROBADA** la escalera de tasas 1→7% · 3→9% · 6→12% · 9→15% · 12→18% | El 1 mes al 7% es petición textual de la empresa; el resto escala monotónico |
| 2026-09-29 | **APROBADO** el orden: ola 7 = cobranza, ola 8 = tablero + novedades | Las gráficas se corrigen bien cuando el saldo ya es por dinero |
| 2026-09-29 | **APROBADA** la campanita con `GET /api/novedades` + comparación de versión (poll, sin push) | Lo más corto que funciona |
| 2026-09-29 | **APROBADO** el modelo de roles: el correo de la empresa es el **admin raíz** y este asigna sub-usuarios por nombre con rol *predeterminado* (subadmin/cajero/vendedor/cobrador); la responsabilidad recae en el usuario, no en quien tenga la cuenta. Diseño en §"Identidad, roles y edición de cartera (ola 9)" | Roles fijos = menos trabajo para el admin y auditoría clara |
| 2026-09-29 | **APROBADO**: el admin de la empresa puede **editar/cancelar créditos de SU cartera** (no los perfiles de cliente, que son de la red y solo PYMZA toca desde soporte). Regla contable: sin pagos → editar/borrar; con pagos → solo cancelar (soft) con motivo, jamás reescribir dinero | Corrige errores de captura sin romper el historial financiero |
| 2026-09-29 | **APROBADO**: score PRIMERO con la red PYMZA (historial propio + recibos); CdC/FICO se integra después. FICO no es fuente de datos: es un modelo que se vende **a través** de los burós (Círculo de Crédito/Buró de Crédito); se contrata el buró y, si se quiere, su score FICO | La red es el foso; el buró es dato externo de pago por consulta (ver `docs/INVESTIGACION.md`) |
| 2026-09-29 | **APROBADO**: se mantiene gratis hasta que la DB valga algo y el producto se sienta pulido (Stripe queda en ola 10 pero condicionado a "producto pulido") | Las empresas son las que están llenando la base |