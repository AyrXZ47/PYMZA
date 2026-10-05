# Auditoría Ola 7 — Cobranza real y cartera usable

- **Fecha:** 2026-10-05 (sesión de auditoría en fresco, árbol integrado `main` @ `952302f`)
- **Alcance:** `.workflow/plan.md` §"Ola 7 (actual)", briefs `wave7-executor-{1,2}.md`, `.workflow/audit-checklist.md`
- **Release gate:** `skills/security-audit` (6 fases) — **cero CRITICAL/HIGH**; 2 MEDIUM documentados como excepciones con owner.
- **Veredicto: APPROVED WITH EXCEPTIONS** (E1, E2 MEDIUM — no bloquean el release gate; owner planner)

---

## 1. Integridad de la integración

| Check | Evidencia | OK |
|---|---|---|
| Worktrees mergeados a main | `git log --oneline 5a40c01..wave7-executor-1` → `3f6ffe2, b6df517, 46dcc31, 6cee610`; `5a40c01..wave7-executor-2` → `a99503f, ea979b7, fe85f70, 1e52c61`. Merges `d0f6a78` (e1) y `103206f` (e2); head integrado `952302f`. `wave7-executor-{1,2}` sin commits fuera de main | ✅ |
| `git status` limpio, sin worktrees/stashes | `git status` → clean; `git stash list` → vacío; `git worktree list` → solo `main`; `main == origin/main` (0/0) | ✅ |
| Nada fuera del mapa de propiedad | `git diff 5a40c01 952302f --name-only` → `backend/src/{db,main,models/credito,pdf,routes/credito}.rs`, `docs/API.md` (e1); `frontend/assets/tailwind.css`, `frontend/src/{api,components/cartera,components/plan_modal}.rs` (e2); `.workflow/plan.md` (planner/integrador). Cero cruces | ✅ |
| Todo lo planeado presente | `main.rs` +2/-1 (solo ruta `/api/creditos/abonos` + import); abonos/tasas/1 mes; índice parcial en `db.rs`; PDF con abonos + sello; `cartera.rs` +576; `plan_modal.rs` opción 1 mes; `api.rs` +87; `docs/API.md` +86 | ✅ |

**Merges `--stat`:** e1 = 606 insertions / 118 deletions en sus 6 archivos; e2 = 651 / 69 en sus 4. Diff total de la ola = 0 archivos fuera de ownership, 0 cambios en `Cargo.toml` (cero deps nuevas).

## 2. Build y tests (árbol integrado)

| Comando | Salida | OK |
|---|---|---|
| `cd backend && cargo build` | `Finished dev profile` | ✅ |
| `cd backend && cargo test` | `81 passed (1 suite, 0 failed)` (incluye los nuevos: abonos/saldo/estado v2/tasas/contrato, regresión `Pago` sin `tipo`) | ✅ |
| `cd frontend && cargo check --target wasm32-unknown-unknown` | `Finished dev profile` | ✅ |
| `cd frontend && cargo test` | `48 passed (1 suite, 0 failed)` (nuevos: `registrar_abono`, `saldo_plan`/`cobrado_plan`/`nombre_plan`, filtros/orden de cartera) | ✅ |
| Verify EJ-2 `./tailwind.sh` → CSS en sync | `tailwindcss v4.3.3 … Done in 92ms`; `git status --porcelain` y `git diff --stat frontend/assets/tailwind.css` → **sin salida** (CSS commiteado byte-idéntico al regenerado) | ✅ |
| Cada brief's verify en el árbol integrado | Cubierto arriba (build+tests) y con el humo de §5 | ✅ |

## 3. Disciplina ponytail

| Check | Evidencia | OK |
|---|---|---|
| Cero deps nuevas | `git diff 5a40c01 952302f -- '*Cargo.toml'` → sin cambios | ✅ |
| Sin abstracciones no pedidas | Cambios concentrados en funciones/handlers existentes; `plan_json`/`estado_plan` extienden el patrón ola 4; no módulos ni capas nuevas | ✅ |
| Menor diff que satisface las tareas | Solo los 11 archivos del mapa; `main.rs` limitado a 2 líneas (import + ruta), como exigía el brief | ✅ |
| `ponytail:` con techo donde se recorta | `cargar_cartera` (memoria vs agregaciones), `resumen_cartera` (ídem), `generar_plan_pagos` (redondeo), `pdf.rs` (truncado a una página vs paginación), `db.rs` (índice no fatal). Presentes y con techo nombrado | ✅ |

## 4. Seguridad — release gate `skills/security-audit`

Fases: recon (arquitectura y trust boundaries del diff), hunt (manual sobre el diff + hunter independiente delegado), validación adversarial (reproducción propia), reporte. Sin CRITICAL/HIGH.

| Check | Evidencia en vivo (mongod + backend locales) | OK |
|---|---|---|
| Sin token → 401 | `POST /api/creditos/abonos` sin header → `401 {"message":"No autorizado…"}` | ✅ |
| Aislamiento de tenant (abono/contrato) | empresa2 `ajena@test.mx`: abono sobre plan de demo → `404 "Plan no encontrado"`; contrato → `404`; su `GET /api/creditos` → `[]` | ✅ |
| Inyección NoSQL | `{"plan_id":{"$ne":null}}` → 422 (serde tipado); queries Mongo usan valores tipados; el plan se busca en memoria por hex | ✅ |
| XSS | Sin `dangerous_inner_html`; `nombre`/`nota` van como texto en `rsx!`; `js_descarga` JSON-escapa el nombre y solo interpola base64 (alfabeto seguro) | ✅ |
| Header injection | `cliente_curp` con `\r\n` en contrato → cae al `filename="contrato.pdf"` por defecto (sin response splitting) | ✅ |
| Entradas del endpoint de abonos | `monto<=0` → 400; `monto>saldo+0.01` → 400; plan liquidado → 400; `1e400` → 400 (body parse); ver §5 | ✅ |
| Cero secretos en el diff | `git diff` de la ola sin URIs/tokens/claves; `git ls-files` solo `.env.example`; `.env` gitignored | ✅ |
| F6 del ledger (TOCTOU cuota) | Índice único parcial `pagos{plan_id,cuota}` `cuota>0` existe y rechaza cuota duplicada (`E11000`), permite varios abonos (`cuota:0`) | ✅ |
| Regresión `Pago` sin `tipo` | Doc legacy insertado sin `tipo`/`nota` → `GET /api/creditos` responde success y calcula cobrado/cuotas/estado; `#[serde(default="tipo_cuota")]` funciona | ✅ |
| Licencias de deps | Cero deps nuevas → sin cambio de licencias. `printpdf` (ola 6) ya auditado | ✅ |

**Nota metodológica (importante):** `backend/.env` tiene `MONGODB_URI` apuntando a **Atlas**. Al levantar el backend para el humo con `cargo run`, dotenvy usó ese URI y las primeras peticiones (solo lectura: login + `evaluar`) tocaron la DB real. Se detectó de inmediato (la CURP del seed no existía) y se reinició con `MONGODB_URI=mongodb://127.0.0.1:27017` explícito; a partir de ahí **todo el humo corrió contra la DB local**. No hubo ningún write en Atlas. Recomendación para futuras sesiones: forzar `MONGODB_URI` local al arrancar el backend de humo (el guard de `seed.js` no protege a `cargo run`).

## 5. Audit gate de la ola 7 (evidencia en vivo, DB local)

Entorno: `mongod --dbpath ~/.mongo-data` local + `mongosh < backend/scripts/seed.js` fresco; backend `target/debug/pymza_backend` con `MONGODB_URI=mongodb://127.0.0.1:27017`. Servicios y DB restaurados/detenidos al final. Cero efectos en producción.

| Check del plan | Resultado | OK |
|---|---|---|
| Plan a 1 mes → tasa | `evaluar {monto:1000, plazo_meses:1}` → `tasa_interes 0.07`, `pago_mensual 1070.00` | ✅ |
| Plazo fuera del catálogo | `plazo_meses:4` → 400 `"El plazo debe ser 1, 3, 6, 9 o 12 meses"` | ✅ |
| Abono parcial no marca cuota | plan deuda 3180 (530×6): abono 200 → `cobrado 200, saldo 2980, cuotas_pagadas 0, estado Activo` | ✅ |
| Liquidación por abonos | abono 2980 → `cobrado 3180, saldo 0, cuotas_pagadas 6, estado Liquidado`; `planes_pago.estado` persistido = `Liquidado` | ✅ |
| Validaciones de abono | `0` → 400 "monto debe ser mayor a 0"; `99999` (>saldo) → 400 "excede el saldo pendiente"; plan liquidado → 400 "ya está liquidado" | ✅ |
| `estado` recalculado en lectura | plan del seed con `estado:"Activo"` persistido y 1 cuota vencida sin cubrir → `GET /api/creditos` responde `Moroso` (usa el recalculado, no el persistido) | ✅ |
| Regresión pago viejo sin `tipo` | doc legacy (cuota 1, $565.33, sin `tipo`/`nota`) → `cobrado 565.33, cuotas_pagadas 1, cuotas_vencidas 1, saldo 2826.65, estado Moroso` | ✅ |
| `nombre` en cartera | `GET /api/creditos` → `nombre:"Janeth Ramos Zamora"` (join `$in` una query) | ✅ |
| Índice parcial | `getIndexes()` → `{plan_id:1,cuota:1}` unique + `partialFilterExpression {cuota:{$gt:0}}`; duplicado cuota 1 → `E11000`; dos `cuota:0` → OK | ✅ |
| Contrato con abonos y sello | plan liquidado por abonos → `200 application/pdf`, header `%PDF-`, hex WinAnsi contiene `Cobrado: $3180.00`, `Pagos y abonos`, `LIQUIDADO — FINIQUITO`; `cobrado + saldo = 3180 = pago_mensual×plazo` | ✅ |
| Humo UI en navegador (buscador/filtros/dos tablas/abono/contrato) | Pendiente del humano (V), como en olas previas; cubierto en lógica por tests | ⏳ owner V |

## Hallazgos y excepciones

### E1 — Carrera de abonos concurrentes: `cobrado` puede exceder la deuda (MEDIUM, owner: planner)
`registrar_abono` (`credito.rs:773-798`) hace read-then-insert sin atomicidad. El índice parcial solo cubre `cuota>0`, así que N abonos concurrentes leen el mismo `cobrado_previo`, pasan todos el guard `monto > saldo+0.01` e insertan.
**Reproducido:** plan deuda 3000 (500×6) + 5 × `POST /api/creditos/abonos {monto:1000}` concurrentes → **4 HTTP 200**; `GET /api/creditos` → `cobrado 4000` (> deuda 3000), `saldo 0`, `estado Liquidado`; 4 documentos en `pagos`.
**Impacto:** corrupción del ledger de cobranza y liquidación prematura. Confinado al propio tenant (requiere JWT válido; hoy todos los usuarios de la empresa son equivalentes). **Severidad MEDIUM** (integridad de datos, no frontera de seguridad cruzada) → no bloquea el release gate.
**Fix sugerido (barato, ola 8):** guard atómico — `update_one` condicional sobre el plan (`$expr` de saldo) antes/dentro del insert, o una operación de reconciliación que rechace si `cobrado+monto > deuda` a nivel Mongo. Un `unique` no aplica (los abonos son legítimamente repetibles). Alternativa mínima: idempotency key por request.

### E2 — `autorizar_credito` confía en `pago_mensual`/`tasa_interes` del body y de ahí deriva la deuda (MEDIUM, owner: planner)
`autorizar` (`credito.rs:561`) valida plazo y `monto_total`, pero persiste `pago_mensual`/`tasa_interes` tal cual llegan. La ola 7 redefine `deuda = pago_mensual × plazo`, así que el monto de la deuda queda controlado por el body.
**Reproducido:** `autorizar {monto_total:100000, plazo_meses:6, pago_mensual:0.01}` → plan con deuda 0.06; `abono {monto:0.06}` → `{monto_total:100000, cobrado:0.06, saldo:0, estado Liquidado}`. El contrato sella `LIQUIDADO — FINIQUITO` sobre un crédito de $100,000.
**Impacto:** registro/contrato de deuda fabricable por la propia empresa (no cruza tenants). Causa raíz preexistente (ola 4/6), impacto elevado por la semántica nueva. **Severidad MEDIUM.**
**Fix sugerido (ola 8):** recomputar `pago_mensual`/`tasa_interes` en el backend desde `monto_total`+`plazo` (fuente de verdad `tasa_por_plazo` + `generar_plan_pagos`), o rechazar si `pago_mensual` no coincide con lo calculado.

### Observaciones (no findings)
- **O1 (LOW):** `resumen_cartera` filtra por `p.estado` **persistido** (`credito.rs:350,364,380`), no por el recalculado; el dashboard puede mostrar morosidad/aging rancios hasta el próximo pago. Ya previsto para la ola 8 ("tablero que dice la verdad").
- **O2 (LOW):** `nota` del abono sin límite de longitud (acotado de facto por el body-limit de 3 MB). 
- **O3 (INFO):** si falla la creación del índice parcial, solo se loguea warning (no fatal) y la carrera F6 se reabre en silencio. Conforme al patrón idempotente/no-fatal ya establecido; el índice se verificó presente en local.

## Veredicto

**APPROVED WITH EXCEPTIONS** — E1 y E2 son **MEDIUM** (integridad de datos dentro del propio tenant, sin cruce de frontera de seguridad), documentados con owner y localizados en el planner. El release gate exige cero CRITICAL/HIGH y no hay ninguno: aislamiento de tenant, auth, validaciones de entrada, ausencia de inyección/XSS/secretos, el índice parcial F6 y la regresión de pagos viejos están verificados en vivo. E1/E2 deben resolverse en la ola 8 (o en un hotfix corto) antes de que la cobranza real acumule volumen; no bloquean el despliegue de la ola 7.

Pendiente pre-ola 8:
- Humo UI en navegador (owner V): buscador, filtros, dos tablas, botón de abono, contrato.
- Cerrar E1 (guard atómico del abono) y E2 (recomputar `pago_mensual`/`tasa_interes` en `autorizar`).
