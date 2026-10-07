# Brief: Wave 8-fix · Executor único (backend)

> Mini-ola de integridad (como la 6-fix). Read `.workflow/plan.md` §"Ola 8-fix
> (actual)" COMPLETE and `.workflow/audits/wave8.md` §"Hallazgos" (E1/E2/O1) before
> writing a single line. One executor: paralelizar no compra nada.

## Task

Cerrar las excepciones de la ola 8 (integridad de dinero en producción) y el
síntoma reportado por V ("un abono muy pequeño mandó una venta de prueba a
liquidados"):

1. **E2-w8 — tope de dinero.** `validar_plazo_y_monto` rechaza `monto_total` no
   finito o `> MONTO_MAX_MXN` (const nueva, p. ej. `1e12`) → 400; y antes de
   persistir en `autorizar`, verifica `pago_mensual_de(...).is_finite()`.
   Nunca más `pago_mensual: Infinity` en Mongo (hoy corrompe el dashboard).
2. **E1-w8 — contador `cobrado` recuperable.** Hoy la reconciliación solo sube
   (`$max`) y el rollback es best-effort: un `$inc` de reserva sin su `Pago` deja
   el plan incobrable para siempre. Añade contadores `reservas` + `reserva_ts` al
   documento del plan (campos BSON crudos, igual que `cobrado`): reservar =
   `$inc {cobrado:monto, reservas:1}`; confirmar (insert del `Pago` OK) =
   `$inc {reservas:-1}`; rollback = `$inc {cobrado:-monto, reservas:-1}`.
   `cargar_cartera` reconcilia contra el ledger en AMBAS direcciones cuando
   `reservas == 0` o la reserva es más vieja que 5 min; planes legacy sin
   `reservas` se tratan como 0 → contador inflado se auto-repara.
3. **O1 — aging/flujo por dinero.** `resumen_cartera` usa `cuotas_cubiertas`
   (dinero) en aging, flujo proyectado y por-cobrar. Un abono de 500 sobre deuda
   3000 debe dar `aging.90+ = 2500`.
4. **Reparación legacy.** `backend/scripts/reparar_planes.js` (dry-run por
   defecto; `--apply` escribe): detecta planes con
   `pago_mensual*plazo != pago_mensual_de(monto_total,plazo)` o `cobrado > ledger`
   y, con `--apply`, recomputa `tasa_interes`/`pago_mensual` desde
   `monto_total`+`plazo` (tabla vigente) y fija `cobrado = ledger`. NO se corre
   en el humo; lo corre V bajo su control.

Esquema ADITIVO: `reservas`/`reserva_ts`/`cobrado` son campos opcionales; los
planes viejos siguen leyéndose. `PlanPago` NO cambia de struct (el `cobrado`
crudo ya se usa así desde la ola 8).

## Definition of done

- E2: `autorizar {monto_total:1e308, plazo_meses:6}` → 400; `1e12+1` → 400; un
  monto normal persiste `pago_mensual` finito y correcto.
- E1: plan con `cobrado:5000` y ledger 0 (sin `reservas`) → tras leer la cartera,
  un `POST /abonos` responde 200 (antes 400 permanente). Con una reserva "muerta"
  (reserva_ts viejo) también se recupera. La concurrencia de la ola 8 sigue:
  5 abonos de 1000 sobre deuda 3000 → 3×200 y 2×400.
- O1: plan deuda 3000 + abono 500 → `aging.90+ = 2500` en `GET /api/creditos/resumen`.
- `reparar_planes.js` sin `--apply` no escribe; con `--apply` corrige un plan de
  prueba inconsistente. Documentado en el encabezado del script.
- Regresión ola 7/8 verde (abono parcial no marca cuota, cuota duplicada 400,
  `Pago` legacy sin `tipo`, contrato PDF, KPIs por ventana). Cero deps nuevas.
- `docs/API.md` actualizado con el tope y la semántica de recuperación.
- The verify command below passes.

## Files you own

- `backend/src/routes/credito.rs`
- `backend/src/models/credito.rs`
- `backend/src/db.rs` (solo si la reconciliación lo necesita)
- `backend/src/main.rs` (solo si hay que registrar algo; idealmente nada)
- `backend/scripts/reparar_planes.js` (NUEVO)
- `docs/API.md`

## Files forbidden

- `backend/src/pdf.rs` (por eso `cobrado` sigue como campo crudo: no tocar el
  literal de sus tests).
- Todo `frontend/**`; `backend/src/auth.rs`, `ocr.rs`, `otp.rs`,
  `routes/cliente.rs`, `models/cliente.rs`.
- `.env*`, `.workflow/**`, `skills/**`, `PIGNUS.md`, `AGENTS.md`, `docs/DEPLOY.md`,
  `Dockerfile.*`, `docker-compose.yml`.
- **La DB real (Atlas).** Fuerza `MONGODB_URI=mongodb://127.0.0.1:27017` en todo
  humo (`backend/.env` apunta a Atlas).

## Read first

- `.workflow/plan.md` §"Ola 8-fix": contrato completo.
- `.workflow/audits/wave8.md` §Hallazgos: E1/E2/O1 con la repro exacta.
- `backend/src/routes/credito.rs`: `validar_plazo_y_monto`, `pago_mensual_de`,
  `reservar_cobrado`, `revertir_reserva`, la reconciliación de `cargar_cartera`,
  `resumen_cartera` (aging/flujo/por-cobrar) y sus tests.
- `backend/src/db.rs`: patrón de script/índice idempotente.
- `backend/scripts/seed.js`: estilo de los scripts existentes.

## Verify command

```bash
cd backend && MONGODB_URI=mongodb://127.0.0.1:27017 cargo build && cargo test

# Humo con mongod local + seed, backend con MONGODB_URI local:
#  E2: autorizar monto_total 1e308 → 400
#  E1: cobrado inflado en Mongo (cobrado:5000, ledger 0) → GET /api/creditos y
#      luego POST /abonos → 200 (y la concurrencia 3×200/2×400 sigue)
#  O1: abono 500 sobre deuda 3000 → resumen aging.90+ = 2500
#  reparar_planes.js (dry-run) lista el plan inconsistente; con --apply lo corrige
```

## Commit

- Conventional commits, una línea, <72 chars, sin atribución de IA ni trailers.
- Commits lógicos separados: `fix(backend): tope de monto_total finito` /
  `fix(backend): contador cobrado recuperable` / `fix(backend): aging por dinero` /
  `chore(backend): script de reparacion de planes legacy`. NUNCA mezcles E1 con E2.
- BRANCH ISOLATION: `git push origin wave8fix-executor-1` tras cada commit. Nunca
  a `main` ni a otra rama; nunca merge/rebase/checkout.

## Report back

- Archivos cambiados, verify con la evidencia de E1 (recuperación), E2 (tope) y O1
  (aging), salida del dry-run del script, desviaciones y preguntas.
