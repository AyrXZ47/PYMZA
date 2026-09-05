# Brief: Wave 6 · Executor 2 — Frontend: descarga de contrato + Docker frontend (API_BASE) + guía de despliegue

> Copia de `.workflow/briefs/_template.md`. El executor no toca archivos que no
> posee, ni "obviamente". Desviaciones → decision log en `.workflow/plan.md`.

## Tarea

Implementar la parte frontend/infra del "Contrato API ola 6" de
`.workflow/plan.md`:

1. **Descarga del contrato**:
   - `api.rs`: `descargar_contrato(plan_id, token)` → `GET
     /api/creditos/{plan_id}/contrato` (patrón `authed_request`, 401 con
     `sesion_ok`) que devuelve los bytes del PDF + `Content-Disposition`.
   - Helper de download en wasm (cfg-gated): crear `Blob` con
     `application/pdf` (web_sys), `URL.createObjectURL`, `<a download>`
     click programático (el nombre de archivo puede venir del header
     `Content-Disposition` o construirse como `contrato-<curp>.pdf`).
     Limpia el object URL. Helper puro para parsear el filename del header,
     testeado en host.
   - `cartera.rs`: botón "Descargar contrato" en cada plan (con estado
     "descargando…"; errores visibles). Si en el modal de autorizar
     (`plan_modal.rs`) es trivial añadirlo al éxito, hazlo; si complica,
     déjalo solo en cartera y documenta el techo.
2. **`Dockerfile.frontend`**: `ARG API_BASE` (default
   `http://127.0.0.1:3000`) → `ENV API_BASE` ANTES del build WASM (dx lo
   captura vía `option_env!` preparado en la ola 2). Verifica leyendo el
   Dockerfile actual y el flujo de build de dx en contenedor; el objetivo es
   que `docker build --build-arg API_BASE=https://api.ejemplo.mx .` produzca
   un wasm que llame a esa URL. Documenta cualquier matiz del build de dx
   (cache, rutas) con `ponytail:`.
3. **`docker-compose.yml` — SOLO el servicio `frontend`**: añadir
   `build: args: - API_BASE=${API_BASE:-http://127.0.0.1:3000}` (el valor
   viene del entorno/compose). NO toques el servicio `backend` ni `mongo`.
4. **`docs/DEPLOY.md`** (nuevo): guía de despliegue en Railway para V:
   - Servicios: backend (repo + Dockerfile.backend; puerto 3000; envs
     `MONGODB_URI`, `JWT_SECRET`, `BIND_ADDR=0.0.0.0:3000`,
     `ALLOWED_ORIGINS=<dominio público del frontend>`, `OCR_LANG=spa`,
     `WHATSAPP_TOKEN`, `WHATSAPP_PHONE_NUMBER_ID`, `WHATSAPP_TEMPLATE`,
     `WHATSAPP_TEMPLATE_LANG`, `RATE_LIMIT_*`), frontend (build arg
     `API_BASE=<url pública del backend>`).
   - Orden de setup: crear proyecto → desplegar backend → copiar dominio
     público → desplegar frontend con ese dominio en `API_BASE` → re-deploy
     si cambió CORS.
   - Verificación post-deploy (curl al login desde el dominio nuevo,
     login en navegador).
   - Backups de Atlas: dónde verlos/verificarlos en la UI (sin código —
     feature del proveedor).
   - Rotación de `JWT_SECRET` (efecto: logout masivo) y troubleshooting
     (CORS bloqueado → revisar ALLOWED_ORIGINS; OCR 500 → revisar
     tesseract en imagen; 429 → rate limit).
   - CERO valores reales de secretos (placeholders tipo `<pegar-aquí>`).
5. **`README.md`**: SOLO añadir una sección/enlace corto "Despliegue" que
   apunte a `docs/DEPLOY.md`. Nada más.
6. Tests en host: parseo del `Content-Disposition` (con y sin filename),
   construcción del body/headers del GET de contrato. Los 36 tests existentes
   deben seguir pasando. CERO deps nuevas.
7. Si el botón añade clases Tailwind nuevas → regenera CSS y commitéalo.

Ponytail: el download es un helper plano con web_sys; sin gestión de estado
global. Marca con `ponytail:` los atajos nombrando su techo.

## Definition of done

- "Descargar contrato" en cartera baja un PDF válido (verificado en el humo
  del plan) y dispara la descarga en navegador.
- `Dockerfile.frontend` acepta `API_BASE` como build arg (evidencia: build
  local con un API_BASE distinto y el wasm que lo contiene — p.ej. `grep`
  sobre el binario, o la instrucción exacta para el auditor).
- `docker-compose.yml` pasa `API_BASE` al build del frontend y nada más
  cambió en el archivo.
- `docs/DEPLOY.md` completo, sin secretos reales.
- El comando de verify de abajo pasa (tests viejos + nuevos).

## Files you own

- `frontend/src/**`
- `frontend/Cargo.toml` (solo si hiciera falta; PROHIBIDO deps nuevas)
- `frontend/tailwind.css`, `frontend/assets/**` (CSS regenerado)
- `Dockerfile.frontend`
- `docker-compose.yml` (SOLO el servicio `frontend`)
- `docs/DEPLOY.md` (nuevo)
- `README.md` (SOLO el enlace a DEPLOY.md)

## Files forbidden

- `backend/**`, `Dockerfile.backend`, `AGENTS.md` (raíz), `docs/API.md`,
  `docs/ROADMAP.md`, `docs/INVESTIGACION.md`, `PYMZA.md`,
  servicios `backend`/`mongo` de `docker-compose.yml`, `.workflow/**`,
  `skills/**`, `backend/.env`

## Read first

- `.workflow/plan.md` — sección "Ola 6 (actual)" (endpoint del contrato y
  build arg del frontend).
- `frontend/src/components/cartera.rs` (lista de planes y badges actuales),
  `frontend/src/api.rs` (helpers, sesión, `API_BASE` ya configurable),
  `frontend/src/components/plan_modal.rs` (éxito de autorizar),
  `Dockerfile.frontend` y `docker-compose.yml` actuales,
  `frontend/AGENTS.md` (web_sys/js_sys desde Dioxus 0.7).

## Verify command

```bash
cd frontend && cargo check --target wasm32-unknown-unknown && cargo test && ./tailwind.sh && git diff --stat assets/tailwind.css
```

## Commit

- MANDATORY: conventional commits, resumen corto, imperativo, una línea
  (`feat:`, `fix:`, `chore:`, `docs:`, `refactor:`, `test:`, `perf:`,
  `style:`, `build:`, `ci:`, `revert:`, scope opcional). <72 chars. Sin
  atribución de IA, sin trailers.
- Un cambio lógico por commit (sugerencia: `feat(frontend): descargar
  contrato pdf`, `build(frontend): api_base como build arg en docker`,
  `docs: guia de despliegue railway`).
- Commitea SOLO tus archivos poseídos.
- BRANCH ISOLATION (obligatorio): commitea y pushea SOLO a tu rama de
  worktree — `git push origin <tu-rama>` — tras cada commit. Nunca a `main`
  ni a otra rama; nunca merge/rebase/fast-forward de ramas ajenas.

## Report back

- Archivos creados/cambiados, salida del verify, instrucción exacta para
  verificar el `API_BASE` inyectado en el build (para el audit gate),
  decisiones de UI (ubicación del botón), y desviaciones del contrato.