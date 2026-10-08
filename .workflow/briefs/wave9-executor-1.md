# Brief: Wave 9 · Executor 1 (backend)

> Lead `.workflow/plan.md` §"Ola 9 (actual)" y `.workflow/audits/wave8.md`
> §Hallazgos (OBS1–OBS5) antes de escribir. Este brief resume; el plan manda.

## Task

Cerrar deuda y habilitar cumplimiento en el backend:

1. **OBS1** — `cargo clippy --all-targets` limpio (hoy 19 warnings: `credito.rs`
   11, `kyc.rs` 4, `otp.rs` 2, `ocr.rs` 1, `pdf.rs` 1). No reportes "0/0" sin pegar
   la salida.
2. **OBS2** — el `update_one` de reconciliación en `cargar_cartera` se condiciona
   en el **filtro** (`reservas: {$in:[0,null]}` **o** `reserva_ts` viejo), no con
   una lectura previa (TOCTOU que puede clobbear una reserva fresca).
3. **OBS3** — una reserva muerta debe limpiarse (`reservas: 0`) **aunque**
   `cobrado == ledger` (hoy el `continue` la deja a ≥1 para siempre).
4. **OBS4** — `bson_i64` acepta también `Bson::Double` (como `bson_f64`).
5. **OBS5** — prefijo literal `ponytail:` en el techo "sin transacciones" de
   `reservar_cobrado`.
6. **`proximos_cobros` por ventana** — `GET /api/dashboard` añade
   `proximos_cobros` = conteo de cuotas cuyo vencimiento cae en `?dias=` (7/15/30,
   default 30) y `monto_proximos_cobros` (suma de `pago_mensual` de esas cuotas),
   excluyendo planes Liquidado/Cancelado. NO es el total futuro.
7. **Evidencia de aceptación de aviso** — `POST /api/empresas` acepta
   `acepta_aviso: bool` + `aviso_version: String`; guarda en el documento de
   `empresas` `{aviso_version, aceptado_en, ip}` (aditivo; la IP sale del
   `ConnectInfo`, nunca del body). Si `acepta_aviso != true` → 400 con mensaje
   claro del contrato.
8. **Rebrand backend** — reemplaza "PYMZA" por "PIGNUS" en strings visibles: PDF
   (`pdf.rs` título y leyenda), `novedades.rs`, mensajes de error y
   `docs/API.md`. NO toques los nombres de colecciones, campos, rutas ni el JWT.
9. **ToS draft** — crea `docs/legal/terminos-de-servicio.md` (borrador para
   revisión de V): partes, servicio, suscripción, limitación de responsabilidad del
   score ("la decisión de otorgar crédito es de la PYME"), uso aceptable, propiedad
   intelectual, datos personales (refiere al aviso), suspensión/terminación,
   jurisdicción Zacatecas.

## Definition of done

- `cargo clippy --all-targets` → 0 warnings, con la salida pegada en el reporte.
- Los 5 casos de OBS probados en vivo: reserva fresca NO se clobbea; reserva muerta
  con `cobrado==ledger` limpia `reservas`; `bson_i64` con Double.
- `GET /api/dashboard?dias=15` y `?dias=30` devuelven conteo/monto de la ventana;
  un plan sin cuotas en la ventana no cuenta; liquidados no cuentan.
- `POST /api/empresas` sin `acepta_aviso` → 400; con `true` guarda
  `aviso_version`/`aceptado_en`/`ip` (verificado en Mongo local).
- `rg -i pymza backend/src docs/API.md` → sin coincidencias visibles (solo
  histórico/comentarios internos si acaso, justificado).
- Zero deps nuevas; `docs/legal/terminos-de-servicio.md` creado.
- Verify command passes.

## Files you own

- `backend/src/**` (incluye `kyc.rs`, `otp.rs`, `ocr.rs`, `pdf.rs` — solo para
  OBS1 y el rebrand)
- `docs/API.md`
- `docs/legal/terminos-de-servicio.md` (NUEVO)

## Files forbidden

- `frontend/**` (executor-2), `aviso-privacidad-integral-2026-10.md` (executor-2),
  `README.md`/`AGENTS.md`/`LICENSE-*` (executor-2).
- `.workflow/**`, `skills/**`, `docs/DEPLOY.md`, `docker-compose.yml`, `.env*`.
- La DB real (Atlas): `MONGODB_URI=mongodb://127.0.0.1:27017` en todo humo.

## Read first

- `.workflow/plan.md` §"Ola 9 (actual)" y §"Ola 8-fix (histórica)".
- `.workflow/audits/wave8.md` §Hallazgos OBS1–OBS5.
- `backend/src/routes/credito.rs`: `cargar_cartera`, `reservar_cobrado`,
  `confirmar_reserva`, `revertir_reserva`, `bson_i64`/`bson_f64`, dashboard.
- `backend/src/routes/empresa.rs` (alta de empresa) y `auth.rs` (uso de
  `ConnectInfo` para IP).
- `backend/src/pdf.rs` y `backend/src/novedades.rs` (strings a rebrandear).

## Verify command

```bash
cd backend && MONGODB_URI=mongodb://127.0.0.1:27017 cargo build && cargo test && cargo clippy --all-targets
# Humo local: dashboard ?dias=15/30; POST /api/empresas con/sin acepta_aviso (Mongo local)
```

## Commit

- Conventional commits, una línea, sin atribución de IA. Commits separados por
  tema (`fix(backend): clippy` / `fix(backend): reconciler` / `feat(backend):
  proximos cobros por ventana` / `feat(backend): evidencia de aviso` /
  `docs: terminos de servicio`).
- BRANCH ISOLATION: `git push origin wave9-executor-1` tras cada commit.

## Report back

- Archivos, salida de clippy y tests, evidencia de OBS1–OBS5, dashboard por
  ventana, alta con evidencia de aviso, y cualquier desviación.
