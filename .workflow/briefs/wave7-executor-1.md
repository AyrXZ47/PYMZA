# Brief: Wave 7 · Executor 1 (backend)

> Copy of the planner's handoff. You never touch a file you don't own, even
> "obviously". Deviations go back to the planner via the decision log in
> `.workflow/plan.md`. Read `.workflow/plan.md` §"Ola 7 (actual)" COMPLETE
> before writing a single line: this brief is a summary, the plan is the source
> of truth.

## Task

Backend de la ola 7: (1) **abonos** — nuevo endpoint `POST /api/creditos/abonos`
que registra un pago parcial sin marcar la cuota como pagada; (2) **tasas** —
`tasa_por_plazo` acepta un plan a 1 mes con 7% y queda escalonada
(`1→0.07, 3→0.09, 6→0.12, 9→0.15, 12→0.18`); (3) **saldo/estado por dinero** —
`deuda = pago_mensual * plazo_meses`, `cobrado` = suma de pagos+abonos,
`saldo = max(0, deuda − cobrado)`, `estado` recalculado SIEMPRE en lectura
(`Liquidado` si saldo ≤ 0.01; `Moroso` si `cuotas_vencidas > 0`;
si no `Activo`); (4) **contrato** — el PDF incluye la sección de abonos/pagos y
el sello `LIQUIDADO — FINIQUITO` cuando saldo ≤ 0.01; (5) **nombre del cliente**
en `GET /api/creditos` con un solo `$in` sobre `clientes`; (6) **índice único
parcial** `pagos{plan_id, cuota}` con `cuota > 0` (cierra F6 del ledger, permite
varios abonos con `cuota: 0`).

Esquema ADITIVO, cero migración: `Pago` gana `tipo: String` con
`#[serde(default = "tipo_cuota")]` (`"cuota"`); los pagos viejos sin el campo
deben dar EXACTAMENTE el mismo saldo/estado que hoy. `PlanPago` no cambia.

Deploy NO es tuyo: V despliega con `docs/DEPLOY.md` tras la auditoría. No toques
Atlas: prueba local (`mongod` + seed demo).

## Definition of done

- `POST /api/creditos/abonos` con la validación ordenada: plan del tenant → 404
  (mismo error único que `registrar_pago` para "no existe" y "no es tuyo");
  `monto` finito y `> 0` → 400; plan `Liquidado` → 400; `monto > saldo + 0.01`
  → 400 con mensaje claro. Inserta `Pago{tipo:"abono", cuota:0, monto, fecha}` y
  responde la shape de `registrar_pago` (plan + `saldo` + `cobrado`).
- `POST /api/creditos/pagos` y `GET /api/creditos/:plan_id/contrato` conservan
  path y body; `GET /api/creditos` conserva la shape y AÑADE `nombre`, `cobrado`,
  `saldo`; `cuotas_pagadas` usa la semántica nueva (cuotas cubiertas por dinero).
- `estado` en la respuesta de `GET /api/creditos` y en los pagos/abonos es el
  RECALCULADO (no el persistido). `upsert_dashboard_stats` usa el recalculado.
- Regresión: un plan del seed (pagos viejos sin `tipo`) reporta el mismo estado y
  `cuotas_pagadas` que hoy.
- PDF: header `%PDF`, sección de abonos/pagos (fecha, tipo, monto), `cobrado`/
  `saldo` de emisión, sello `LIQUIDADO — FINIQUITO` cuando saldo ≤ 0.01.
- Índice parcial creado en `db.rs::connect` (idempotente, no fatal): con él, un
  segundo pago de la MISMA cuota falla en Mongo; varios abonos (`cuota: 0`) pasan.
- Tests puros nuevos (sin DB): saldo, cuotas cubiertas, estado v2, tasas nuevas,
  validación de plazos, contrato con abonos. Y `docs/API.md` actualizado.
- The verify command below passes.

## Files you own

- `backend/src/routes/credito.rs`
- `backend/src/models/credito.rs`
- `backend/src/pdf.rs`
- `backend/src/db.rs`
- `backend/src/main.rs` (ÚNICAMENTE: registrar la ruta `/api/creditos/abonos` y
  su import; nada más)
- `docs/API.md`
- `backend/scripts/**` (solo si necesitas un fixture/comando de humo)

## Files forbidden

- TODO `frontend/**` (executor-2), en especial `frontend/src/api.rs`,
  `cartera.rs`, `plan_modal.rs`, `assets/tailwind.css`.
- `backend/src/routes/cliente.rs`, `backend/src/models/cliente.rs` (el join a
  `clientes` se hace desde `credito.rs` con el struct `Cliente` existente).
- `backend/src/auth.rs`, `backend/src/ocr.rs`, `backend/src/otp.rs`,
  `backend/src/routes/*` que no sean los tuyos.
- `.env*`, `.workflow/**`, `skills/**`, `PYMZA.md`, `AGENTS.md`, `docs/DEPLOY.md`,
  `docs/ROADMAP.md`, `docs/INVESTIGACION.md`, `Dockerfile.*`,
  `docker-compose.yml`.
- La DB de producción (Atlas). Jamás.

## Read first

- `.workflow/plan.md` → §"Ola 7 (actual)": semántica de deuda/saldo/estado y
  contrato API. Es la autoridad.
- `backend/src/routes/credito.rs`: `generar_plan_pagos`, `fecha_vencimiento`,
  `estado_plan`, `cuotas_vencidas`, `cargar_cartera`, `PagosPlan`, `plan_json`,
  `registrar_pago`, `upsert_dashboard_stats`, `obtener_resumen`, tests inline.
- `backend/src/models/credito.rs`: `Pago`, `PlanPago`, `RegistrarPagoReq`.
- `backend/src/pdf.rs`: firma de `pdf_contrato` y layout por milímetros.
- `backend/src/db.rs`: patrón de índice idempotente (`crear_indice_unico_empresa_correo`).
- `backend/src/main.rs`: patrón de rutas protegidas y el 3MB body limit.
- `docs/API.md`: contrato documentado que debes actualizar.

## Verify command

```bash
# 1. Build + tests (los tests puros nuevos cubren saldo/estado/tasas/contrato)
cd backend && cargo build && cargo test

# 2. Humo contra mongod LOCAL (nunca Atlas). En otra terminal: mongod corriendo
#    en 127.0.0.1:27017 con el seed demo, y `cargo run`.
TOKEN=$(curl -s -X POST http://127.0.0.1:3000/api/login -H 'content-type: application/json' \
  -d '{"correo":"demo@pymza.mx","password":"demo1234"}' | jq -r .token)
# 2a. plan a 1 mes: tasa 0.07
curl -s http://127.0.0.1:3000/api/creditos/evaluar -H "Authorization: Bearer $TOKEN" \
  -H 'content-type: application/json' -d '{"curp":"<curp-del-seed>","monto":1000,"plazo_meses":1}' \
  | jq '.tasa_interes'          # → 0.07
# 2b. abono parcial → saldo baja, la cuota NO se marca
# 2c. abono > saldo, abono ≤ 0, plan liquidado → 400 · plan ajeno → 404 · sin token → 401
# 2d. contrato de plan con abonos → %PDF · de plan liquidado → contiene LIQUIDADO
```

## Commit

- MANDATORY: conventional commits, short summary, imperative, one line
  (`feat:`, `fix:`, `docs:`, `refactor:`, `test:`), <72 chars, no AI
  attribution, no trailers.
- Puedes hacer varios commits lógicos (p. ej. `feat(backend): abonos ...` y
  `feat(backend): contrato con abonos ...`); NUNCA un commit que mezcle abonos
  con el contrato.
- Commit ONLY your owned files. Si crees que otro archivo necesita cambio,
  repórtalo, no lo toques.
- BRANCH ISOLATION (mandatory): `git push origin wave7-executor-1` después de
  cada commit. Nunca a `main` ni a otra rama; nunca merge/rebase/checkout.

## Report back

- Archivos cambiados, salida del verify command, decisiones de semántica que
  tomaste, cualquier desviación del plan, y preguntas abiertas (por ejemplo si
  el conjunto de plazos válidos rompe algún flujo que no viste).
