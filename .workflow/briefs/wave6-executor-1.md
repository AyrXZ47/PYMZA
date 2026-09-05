# Brief: Wave 6 · Executor 1 — Backend: contrato PDF + hardening (CORS, body limit, rate limit) + Docker backend

> Copia de `.workflow/briefs/_template.md`. El executor no toca archivos que no
> posee, ni "obviamente". Desviaciones → decision log en `.workflow/plan.md`.

## Tarea

Implementar la parte backend del "Contrato API ola 6" de `.workflow/plan.md`:

1. **Contrato PDF**:
   - `backend/src/pdf.rs` (nuevo): función pura
     `pdf_contrato(empresa_nombre, empresa_correo, cliente_nombre, cliente_curp,
     plan, fecha) -> Vec<u8>` con `printpdf` (ÚNICA dep nueva de esta pieza;
     revisa cuál es la versión estable actual — 0.7/0.8) usando Helvetica
     base14 (latin1: acentos ok). Contenido: título "CONTRATO DE CRÉDITO",
     fecha de emisión, datos de empresa (nombre, correo), cliente (nombre,
     CURP), producto, monto total, plazo, tasa de interés, tabla completa de
     pagos (mes, pago, interés, capital, saldo — del `plan_pagos` del plan o
     regenerada con la misma fórmula), línea de firma, leyenda "Contrato
     generado por PYMZA". Diseño simple y legible (posiciones de texto; sin
     imágenes ni logos).
   - `GET /api/creditos/{plan_id}/contrato` (protegido, `EmpresaSession`) en
     `routes/credito.rs`: parsea el `plan_id` hex → ObjectId (inválido →
     400); plan existe y `plan.empresa == sesion.correo` → si no, 404; lookup
     de empresa (nombre) y cliente (nombre por curp) → `pdf_contrato` →
     respuesta con `Content-Type: application/pdf` y header
     `Content-Disposition: attachment; filename="contrato-<curp>.pdf"`
     (Axum 0.6: `IntoResponse` con `AppendHeaders`/typed header o builder —
     el patrón de la app; documenta con `ponytail:` si simplificas).
   - Test: `pdf_contrato` produce bytes que empiezan con `%PDF-` y tamaño
     > 1KB; acentos presentes (buscar el patrón latin1 del título con
     acento, p.ej. "CRÉDITO", en los bytes no es trivial con compresión —
     printpdf base14 sin comprimir streams suele dejar texto legible; si el
     test del acento resulta frágil, testea título sin acento y documenta).
2. **CORS productivo**: `cors_layer` en `auth.rs` — orígenes desde env
   `ALLOWED_ORIGINS` (separada por comas); default
   `http://localhost:8080,http://127.0.0.1:8080` (dev actual, sin cambio de
   comportamiento si la env no está). Función de parseo pura + tests.
3. **Body limit (cierra E1 del auditor ola 5)**: en `main.rs`,
   `.layer(DefaultBodyLimit::max(3_000_000))` (axum::extract::DefaultBodyLimit)
   — el handler de kyc/recibos YA rechaza >2MB con el 400 del contrato; el
   413 desaparece para archivos de hasta 3MB de b64. Nota de test o evidencia
   en el humo (el test unitario del límite del handler ya existe).
4. **Rate limiting por IP (rutas públicas)**: crate `tower-governor` (dep
   nueva; verifica compatibilidad con axum 0.6 — versión 0.3/0.4) sobre
   `POST /api/login` y `POST /api/empresas`: p. ej. 10 req por 60s por IP →
   429 con JSON `{status:"error", message:"Demasiadas peticiones, intenta más
   tarde"}`. Config por env `RATE_LIMIT_RPS`/`RATE_LIMIT_BURST` con defaults
   razonables (10/20). Funciones puras para la construcción de la capa.
   El error de tower-governor debe convertirse en el JSON de arriba (middleware
   de mapeo) — no un 500 crudo.
5. **Fixture recibos (cierra E2)**: `backend/scripts/fixture_recibo.png` —
   PNG ~600×800 estilo recibo con texto grande: encabezado "RECIBO DE
   SERVICIO", líneas de servicio/periodo y `TOTAL: $450.00 MXN` bien legible.
   Verifica con tesseract (`tesseract ... -l spa --psm 6`) que el output
   contiene el monto y que `buscar_monto` extrae `450.00` — pega el output en
   tu reporte.
6. **`Dockerfile.backend`**: instalar `tesseract-ocr` + `tesseract-ocr-spa`
   (y `libssl-dev`/ca-certificates si el runtime actual no los trae — el
   binario de mongodb driver con openssl-tls lo necesita en runtime). Mantén
   la imagen multi-stage si ya lo es. NO toques `docker-compose.yml` (otro
   dueño).
7. **docs/API.md**: endpoint del contrato, CORS env, rate limit (429), envs
   nuevas. **.env.example**: `ALLOWED_ORIGINS=""`, `RATE_LIMIT_RPS=""`,
   `RATE_LIMIT_BURST=""`.
8. Tests: `pdf_contrato` (header/tamaño), parseo `ALLOWED_ORIGINS` (vacía,
   con espacios, múltiples), rate limit (la config pura; el 429 end-to-end
   queda para el humo si tower-governor lo permite en tests — no fuerces
   infra de red en unit tests). Los 60 tests existentes deben seguir pasando.

Ponytail: PDF con texto posicionado, sin motores de layout; rate limit solo
donde importa. Marca con `ponytail:` los atajos nombrando su techo.

## Definition of done

- `GET .../contrato` devuelve `%PDF` válido con los datos del tenant; plan
  ajeno → 404; sin token → 401; plan_id inválido → 400.
- `ALLOWED_ORIGINS` controla los orígenes (default dev sin cambio).
- Archivo kyc/recibos de hasta ~3MB de b64 → 400 del handler (no 413).
- 11º login seguido desde la misma IP → 429 con JSON claro.
- `fixture_recibo.png` legible por tesseract con monto extraíble.
- Dockerfile backend con tesseract (verificable en el humo Docker del plan).
- El comando de verify de abajo pasa (tests viejos + nuevos).

## Files you own

- `backend/Cargo.toml` (solo `printpdf` y `tower-governor`)
- `backend/src/**`
- `backend/scripts/**` (fixture_recibo.png)
- `docs/API.md`
- `.env.example`
- `Dockerfile.backend`

## Files forbidden

- `frontend/**`, `docker-compose.yml`, `Dockerfile.frontend`,
  `docs/DEPLOY.md`, `README.md`, `AGENTS.md` (raíz), `docs/ROADMAP.md`,
  `docs/INVESTIGACION.md`, `PYMZA.md`, `.workflow/**`, `skills/**`,
  `backend/.env`

## Read first

- `.workflow/plan.md` — sección "Ola 6 (actual)" (contrato PDF, hardening,
  E1/E2 a cerrar).
- `backend/src/main.rs` (wiring, layers), `backend/src/auth.rs` (cors_layer),
  `backend/src/routes/credito.rs` (handlers de créditos y shapes),
  `backend/src/ocr.rs` (buscar_monto para el fixture), `backend/src/models/credito.rs`
  (PlanPago), `backend/scripts/fixture_ine.png` (patrón para el fixture nuevo).

## Verify command

```bash
cd backend && cargo build && cargo test
```

## Commit

- MANDATORY: conventional commits, resumen corto, imperativo, una línea
  (`feat:`, `fix:`, `chore:`, `docs:`, `refactor:`, `test:`, `perf:`,
  `style:`, `build:`, `ci:`, `revert:`, scope opcional). <72 chars. Sin
  atribución de IA, sin trailers.
- Un cambio lógico por commit (sugerencia: `feat(backend): contrato pdf por
  plan`, `feat(backend): cors por env y body limit`, `feat(backend): rate
  limit en rutas publicas`, `fix(backend): fixture recibo y docker tesseract`).
- Commitea SOLO tus archivos poseídos.
- BRANCH ISOLATION (obligatorio): commitea y pushea SOLO a tu rama de
  worktree — `git push origin <tu-rama>` — tras cada commit. Nunca a `main`
  ni a otra rama; nunca merge/rebase/fast-forward de ramas ajenas.

## Report back

- Archivos creados/cambiados, salida de `cargo test` (número), versión de
  `printpdf`/`tower-governor` usadas y su compatibilidad con axum 0.6, output
  del tesseract sobre `fixture_recibo.png`, y desviaciones del contrato.