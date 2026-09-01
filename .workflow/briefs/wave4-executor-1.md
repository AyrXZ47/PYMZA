# Brief: Wave 4 · Executor 1 — Backend: pagos, estados de plan, resumen para gráficas, plantilla WhatsApp

> Copia de `.workflow/briefs/_template.md`. El executor no toca archivos que no
> posee, ni "obviamente". Desviaciones → decision log en `.workflow/plan.md`.

## Tarea

Implementar EXACTAMENTE el "Contrato API ola 4" de `.workflow/plan.md`:

1. **Colección `pagos`** (nueva, en `models/credito.rs` o `models/pago.rs`):
   `{ plan_id, empresa, cliente_curp, cuota, monto, fecha }`.
2. **`POST /api/creditos/pagos`** (protegido, `EmpresaSession`): body
   `{ plan_id, cuota, monto }` (plan_id = hex del ObjectId del plan).
   Validaciones en orden: plan existe (404) y `plan.empresa == sesion.correo`
   (404/403 con mensaje claro); cuota en `1..=plazo_meses` (400); cuota no
   pagada ya — query a `pagos` por plan_id+cuota (400 "Cuota ya registrada");
   monto == `pago_mensual` del plan con tolerancia 0.01 (400). Inserta y
   recalcula el estado del plan (punto 3). Devuelve
   `{status: "success", plan: <plan actualizado con cuotas_pagadas>}`.
3. **Ciclo de vida del plan** (funciones puras testeadas en
   `routes/credito.rs` o módulo propio):
   - `fecha_vencimiento(plan, n)` = fecha del plan + n meses.
   - estado nuevo tras registrar pago: si cuotas pagadas == plazo_meses →
     `Liquidado`; si existe cuota vencida (fecha_vencimiento < hoy) sin pago
     → `Moroso`; si no → `Activo`. Persistir con update del campo `estado`.
   - `cuotas_pagadas(plan)` = count de pagos; `cuotas_vencidas(plan)` =
     vencidas − pagadas (≥0).
   - `autorizar_credito` sigue insertando con `estado: "Activo"`, pero
     captura el `inserted_id` y lo INCLUYE en la respuesta y en el doc (o
     garantiza que `GET /api/creditos` exponga el `_id` serializado como hex
     string — el frontend lo necesita para registrar pagos).
4. **`GET /api/creditos`**: añadir `cuotas_pagadas` y `cuotas_vencidas` a cada
   plan de la respuesta (calculadas, no persistidas).
5. **`GET /api/creditos/resumen`** (nuevo, protegido): calcula y devuelve la
   shape EXACTA del contrato (`cobrado_vs_por_cobrar` 6 meses,
   `tasa_morosidad`, `flujo_proyectado` 30/60/90, `aging` 4 buckets,
   `top_deudores` máx 10 con nombre, `distribucion_montos` 3 buckets).
   Definiciones exactas en el plan. Funciones puras para buckets y
   vencimientos, con tests. El join con `clientes` para el nombre del deudor
   es un lookup por curp (en memoria si el volumen lo permite — ponytail).
6. **`dashboard_stats`** (upsert de `autorizar` y recalculo al registrar
   pago): `creditos_activos` = count planes con estado Activo o Moroso;
   `proximos_cobros` = cuotas que vencen en ≤30 días de planes no liquidados.
   No romper el shape actual (`{empresa, creditos_activos, capital_prestado,
   proximos_cobros}`).
7. **`WhatsAppOtpSender` → plantilla de autenticación** (en `otp.rs`):
   payload `type: "template"` con `template.name` = env `WHATSAPP_TEMPLATE`
   (default `pymza_otp_verification`), `language.code` = env
   `WHATSAPP_TEMPLATE_LANG` (default `es`), `components: [{type: "body",
   parameters: [{type: "text", text: codigo}]}]`. Builder del payload como
   función PURA con test. Si el envío falla → eprintln y continuar (sin
   panic). `WHATSAPP_WABA_ID` NO se usa (documentar en comment si hace falta).
8. **TTL index** (O2 auditor ola 3): al conectar la DB (o lazy en el primer
   `solicitar`), crear índice en `verificaciones.expira_en` con
   `expireAfterSeconds: 0`. Idempotente (create_index es no-op si existe).
9. **docs/API.md**: los 2 endpoints nuevos/extendidos + campos nuevos.
   **.env.example**: `WHATSAPP_TEMPLATE=""` y `WHATSAPP_TEMPLATE_LANG=""`.
10. Tests: vencimientos/estados (cuota pagada a tiempo, atrasada, plan
    liquidado), buckets aging/distribución, shape del resumen, payload de la
    plantilla. Los 23 tests existentes deben seguir pasando.

Ponytail: sin framework de agregación — un handler que lee planes + pagos del
tenant y calcula en memoria es suficiente para el volumen de una PYME. Marca
con `ponytail:` cualquier atajo nombrando su techo.

## Definition of done

- Pago válido registra, recalcula estado y devuelve el plan actualizado.
- Duplicado → 400; cuota fuera de rango → 400; monto ≠ pago_mensual → 400;
  plan de otro tenant → 404.
- `GET /api/creditos` expone `_id` + `cuotas_pagadas`/`cuotas_vencidas`.
- `GET /api/creditos/resumen` devuelve la shape exacta del contrato.
- Plantilla WhatsApp con builder puro testado; TTL index creado.
- El comando de verify de abajo pasa (tests viejos + nuevos).

## Files you own

- `backend/Cargo.toml` (no deberías necesitar tocarlo: CERO deps nuevas)
- `backend/src/**`
- `backend/scripts/**`
- `docs/API.md`
- `.env.example`

## Files forbidden

- `frontend/**`, `AGENTS.md` (raíz), `README.md`, `docs/ROADMAP.md`,
  `docs/INVESTIGACION.md`, `PYMZA.md`, `docker-compose.yml`, `Dockerfile.*`,
  `.workflow/**`, `skills/**`, `backend/.env` (secreto del humano)

## Read first

- `.workflow/plan.md` — sección "Ola 4 (actual)" y su Contrato API (shape
  exacta del resumen).
- `backend/src/routes/credito.rs` (autorizar/obtener_creditos/obtener_dashboard
  actuales), `backend/src/models/credito.rs`, `backend/src/otp.rs`
  (WhatsAppOtpSender), `backend/src/main.rs` (wiring), `backend/src/auth.rs`
  (`EmpresaSession`).

## Verify command

```bash
cd backend && cargo build && cargo test
```

## Commit

- MANDATORY: conventional commits, resumen corto, imperativo, una línea
  (`feat:`, `fix:`, `chore:`, `docs:`, `refactor:`, `test:`, `perf:`,
  `style:`, `build:`, `ci:`, `revert:`, scope opcional). <72 chars. Sin
  atribución de IA, sin trailers.
- Un cambio lógico por commit (sugerencia: `feat(backend): registro de pagos
  y estados de plan`, `feat(backend): resumen de cartera para graficas`,
  `feat(backend): plantilla auth whatsapp y ttl index`).
- Commitea SOLO tus archivos poseídos.
- BRANCH ISOLATION (obligatorio): commitea y pushea SOLO a tu rama de
  worktree — `git push origin <tu-rama>` — tras cada commit. Nunca a `main`
  ni a otra rama; nunca merge/rebase/fast-forward de ramas ajenas.

## Report back

- Archivos creados/cambiados, salida de `cargo test` (número de tests), el
  shape final del resumen (si difiere en algo del plan, por qué), y
  desviaciones del contrato.