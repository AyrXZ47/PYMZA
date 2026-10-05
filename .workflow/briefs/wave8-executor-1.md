# Brief: Wave 8 · Executor 1 (backend)

> Copy of the planner's handoff. You never touch a file you don't own, even
> "obviously". Deviations go back to the planner via the decision log in
> `.workflow/plan.md`. Read `.workflow/plan.md` §"Ola 8 (actual)" COMPLETE and
> `.workflow/audits/wave7.md` (E1/E2/O1) before writing a single line: this brief
> is a summary, the plan is the source of truth.

## Task

Backend de la ola 8, tres frentes:

1. **E1 — abono atómico (MEDIUM del auditor).** Hoy `registrar_abono` hace
   read-then-insert sin atomicidad: 5 abonos concurrentes de 1000 sobre una deuda
   de 3000 pasan todos y dejan `cobrado=4000`. Fix: `PlanPago` gana
   `cobrado: f64` (`#[serde(default)]`, legacy → 0.0) como contador operativo;
   `cargar_cartera` lo reconcilia con la suma del ledger (best-effort) y
   `registrar_abono`/`registrar_pago` RESERVAN atómicamente con
   `find_one_and_update` + guard `$expr` `cobrado+monto <= pago_mensual*plazo`;
   si no matchea → 400; si el insert del `Pago` falla → `$inc` inverso y 500.
   Sin transacciones (funciona en mongod standalone). `nota` acotada a 280 chars.
2. **E2 — `autorizar` recalcula montos (MEDIUM).** No confiar en
   `pago_mensual`/`tasa_interes` del body: recomputar con
   `tasa_por_plazo(plazo)` y `round(monto_total*(1+tasa)/plazo, 2)` y persistir
   los recalculados (el body sigue aceptando los campos por compat, se ignoran).
3. **Tablero honesto + novedades.** `GET /api/dashboard` y
   `GET /api/creditos/resumen` aceptan `?desde&hasta` y calculan KPIs/series en
   vivo; `resumen_cartera` deja de usar el `estado` PERSISTIDO (bug O1) y usa el
   recalculado de la ola 7. `GET /api/novedades` (pública) devuelve
   `{version, novedades}` desde `backend/src/novedades.rs`.

No cambies la semántica de deuda/saldo de la ola 7 (`deuda = pago_mensual*plazo`,
`saldo = max(0, deuda − cobrado)`, estado recalculado): la ola 8 la hace confiable.

## Definition of done

- E1 cerrado: la repro del auditor (plan deuda 3000 + 5 `POST /abonos {monto:1000}`
  concurrentes) da **exactamente 3×200 y 2×400**, `cobrado == 3000`, `saldo == 0`.
- E1 no rompe la ola 7: abono parcial no marca cuota; el pago de cuota exacta
  sigue rechazando duplicados (E11000 del índice parcial → rollback del `$inc`).
- E2 cerrado: `autorizar {monto_total:100000, plazo_meses:6, pago_mensual:0.01,
  tasa_interes:0}` persiste deuda correcta (no 0.06) con la tasa de
  `tasa_por_plazo(6)=0.12`.
- `GET /api/dashboard?desde&hasta` devuelve `capital_colocado`,
  `cobrado_periodo`, `por_cobrar_neto`, `cartera_vencida`, `tasa_morosidad`
  (dinero: `cartera_vencida/capital_colocado`) calculados en vivo desde la
  cartera; conserva los 3 campos viejos.
- `GET /api/creditos/resumen?desde&hasta`: la serie `cobrado_vs_por_cobrar`
  respeta la ventana; aging/morosidad/flujo/top/distribución usan el estado
  RECALCULADO.
- `GET /api/novedades` sin token → `200 {version, novedades:[...]}`; sin secretos.
- Tests puros nuevos donde aplique (reconciliación/serie por ventana) y
  `docs/API.md` actualizado. The verify command below passes.

## Files you own

- `backend/src/routes/credito.rs`
- `backend/src/models/credito.rs`
- `backend/src/db.rs` (solo si el reconciler lo necesita)
- `backend/src/novedades.rs` (NUEVO: const `VERSION` + changelog + handler)
- `backend/src/main.rs` (montar `/api/novedades`; nada más)
- `docs/API.md`

## Files forbidden

- TODO `frontend/**` (executor-2).
- `backend/src/routes/cliente.rs`, `backend/src/models/cliente.rs`,
  `backend/src/auth.rs`, `backend/src/ocr.rs`, `backend/src/otp.rs`,
  `backend/src/pdf.rs` (el contrato ya cubre abonos desde la ola 7).
- `.env*`, `.workflow/**`, `skills/**`, `PYMZA.md`, `AGENTS.md`, `docs/DEPLOY.md`,
  `docs/ROADMAP.md`, `docs/INVESTIGACION.md`, `Dockerfile.*`, `docker-compose.yml`.
- **La DB de producción (Atlas). Jamás.** `backend/.env` apunta a Atlas: fuerza
  `MONGODB_URI=mongodb://127.0.0.1:27017` en todo humo.

## Read first

- `.workflow/plan.md` §"Ola 8 (actual)": contrato completo.
- `.workflow/audits/wave7.md` §"Hallazgos": E1, E2 y O1 con la repro exacta.
- `backend/src/routes/credito.rs`: `cargar_cartera`, `PagosPlan`,
  `registrar_pago`, `registrar_abono`, `estado_plan`/saldo (ola 7),
  `resumen_cartera`, `obtener_resumen`, `obtener_dashboard`,
  `upsert_dashboard_stats`, tests inline.
- `backend/src/models/credito.rs`: `Pago`, `PlanPago`, `RegistrarPagoReq`,
  `DashboardStats`.
- `backend/src/db.rs`: patrón de índice idempotente/no fatal.
- `backend/src/main.rs`: Router, rutas públicas con/sin rate limit.

## Verify command

```bash
# 1. Build + tests (DB local SIEMPRE)
cd backend && MONGODB_URI=mongodb://127.0.0.1:27017 cargo build && cargo test

# 2. Humo con mongod LOCAL + seed, backend con MONGODB_URI local
TOKEN=$(curl -s -X POST http://127.0.0.1:3000/api/login -H 'content-type: application/json' \
  -d '{"correo":"demo@pymza.mx","password":"demo1234"}' | jq -r .token)
# E1: plan deuda 3000 → 5 abonos de 1000 concurrentes → 3×200, 2×400
# E2: autorizar con pago_mensual/tasa falsos → deuda recalculada correcta
# dashboard ?desde&hasta → cobrado_periodo/cartera_vencida correctos
# resumen → estados recalculados (sin "Activo" rancio)
# novedades: curl -s http://127.0.0.1:3000/api/novedades | jq .version
```

## Commit

- MANDATORY: conventional commits, short summary, imperative, one line, <72
  chars, no AI attribution, no trailers.
- Commits lógicos separados: `fix(backend): abono atomico ...`,
  `fix(backend): autorizar recalcula montos`, `feat(backend): tablero por periodo ...`,
  `feat(backend): endpoint de novedades`. NUNCA mezcles E1 con E2.
- Commit ONLY your owned files. Reporta cualquier otro archivo que creas que
  necesita cambio, no lo toques.
- BRANCH ISOLATION (mandatory): `git push origin wave8-executor-1` después de cada
  commit. Nunca a `main` ni a otra rama; nunca merge/rebase/checkout.

## Report back

- Archivos cambiados, salida del verify command (especialmente la repro de E1 y
  E2), decisiones de semántica de KPIs, desviaciones del plan y preguntas.
