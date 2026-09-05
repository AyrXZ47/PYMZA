# Brief: Wave 5 · Executor 2 — Frontend: panel KYC (INE) + score por recibos + badges

> Copia de `.workflow/briefs/_template.md`. El executor no toca archivos que no
> posee, ni "obviamente". Desviaciones → decision log en `.workflow/plan.md`.

## Tarea

Implementar la parte frontend del "Contrato API ola 5" de
`.workflow/plan.md`:

1. **`api.rs`**:
   - `archivo_a_b64(bytes: &[u8]) -> String` (crate `base64 = "0.22"` — única
     dep nueva permitida) — función pura, testeable en host.
   - `kyc_verificar(curp, archivo_b64, mime, token)` → `POST
     /api/clientes/{curp}/kyc` con body `{archivo_b64, mime}`.
   - `recibo_subir(curp, archivo_b64, mime, tipo, token)` → `POST
     /api/clientes/{curp}/recibos` con body `{archivo_b64, mime, tipo}`.
   - Manejo de errores con `sesion_ok` (401 → logout) como el resto.
2. **Lectura de archivo en WASM**: en el componente (o helper de `api.rs`
   cfg-gated), el `<input type="file">` con `onchange` →
   `event.files()` (Dioxus 0.7) → leer bytes del `File` con web_sys/js_sys
   (async, `spawn`) → `archivo_a_b64` → POST. Validación client-side mínima:
   tamaño ≤ 2 MB y mime de imagen (no mandes a fallar al server algo que
   puedes detectar antes). Ver `frontend/AGENTS.md` para el API exacto de
   `onchange` y `Event::Files` en 0.7.
3. **Panel "Verificar INE" (KYC)** — en `alta_cliente.rs` (o nuevo componente
   `kyc.rs` que alta_cliente y búsqueda rendericen; tu elección, documenta):
   - Visible cuando hay un cliente cargado (alta exitosa o búsqueda) sin
     `ine_verificada`.
   - Input file (accept: image/png,image/jpeg,image/webp) + botón "Verificar
     INE". Estados: leyendo archivo, enviando, resultado (coincide → badge
     "✓ INE verificada" verde + `curp_ine` leída; no coincide → mensaje
     ámbar "La CURP de la INE (X) no coincide con la capturada (Y)"; CURP no
     legible → "No se pudo leer la CURP del documento, prueba otra foto";
     OCR no disponible → mensaje claro).
   - Si `ine_verificada` es true, mostrar el badge directamente sin panel.
4. **Panel "Score por recibos"**: select tipo (Luz/Agua/Teléfono) + input
   file + botón "Subir recibo". Resultado: score nuevo + nivel de riesgo +
   recibos contados (X/2). El 3er intento muestra el error del backend
   ("Máximo 2 recibos por cliente").
5. **Búsqueda por CURP** (panel existente): junto al badge de teléfono
   verificado (ola 3), añadir badge "✓ INE" cuando `ine_verificada: true`, y
   mostrar `score` + `nivel_riesgo` del cliente en el resultado.
6. Estilos con los pares light/dark existentes (patrón de la ola 2:
   `bg-white dark:bg-slate-900` etc.). CERO deps nuevas salvo `base64`.
   Regenera el CSS con `./tailwind.sh` si añades clases nuevas y commitea
   `frontend/assets/tailwind.css`.
7. Tests en host: `archivo_a_b64` (consoante al estándar base64), parseo de
   las respuestas kyc/recibos (shapes del contrato), y lógica del panel
   (semáforo de estados). Los 22 tests existentes deben seguir pasando.

Ponytail: sin componente de "upload wizard" genérico; dos bloques condicionales
dentro del panel existente. Marca con `ponytail:` los atajos nombrando su techo.

## Definition of done

- Subir la INE (fixture del backend o imagen real) desde la UI → resultado
  visible (coincide/no) y badge persistente en búsqueda.
- Subir recibo → score nuevo y nivel visibles; tope de 2 con error claro.
- Archivo >2MB o mime no-imagen → error client-side sin llamar al backend.
- Los 22 tests existentes + los nuevos pasan. El comando de verify de abajo
  pasa (incluye CSS en sync).

## Files you own

- `frontend/src/**`
- `frontend/Cargo.toml` (SOLO añadir `base64 = "0.22"`)
- `frontend/tailwind.css` (solo si añades clases nuevas)
- `frontend/assets/**` (tailwind.css regenerado)

## Files forbidden

- `backend/**`, `AGENTS.md` (raíz), `README.md`, `docs/**`
- `frontend/tailwind.sh`, `frontend/Dioxus.toml`, `frontend/AGENTS.md`
- `PYMZA.md`, `docker-compose.yml`, `Dockerfile.*`, `.workflow/**`, `skills/**`

## Read first

- `.workflow/plan.md` — sección "Ola 5 (actual)" y Contrato API (shapes de
  respuesta — tus structs serde).
- `frontend/AGENTS.md` — Dioxus 0.7: manejo de `onchange` de input file,
  `Event::Files`, `spawn`, señales antes de `await`.
- `frontend/src/components/alta_cliente.rs` (panel y búsqueda actuales, el
  badge de teléfono de la ola 3), `frontend/src/api.rs` (helpers, sesión),
  `frontend/src/components/login.rs` (patrón de estados de un flujo).

## Verify command

```bash
cd frontend && cargo check --target wasm32-unknown-unknown && cargo test && ./tailwind.sh && git diff --stat assets/tailwind.css
```

## Commit

- MANDATORY: conventional commits, resumen corto, imperativo, una línea
  (`feat:`, `fix:`, `chore:`, `docs:`, `refactor:`, `test:`, `perf:`,
  `style:`, `build:`, `ci:`, `revert:`, scope opcional). <72 chars. Sin
  atribución de IA, sin trailers.
- Un cambio lógico por commit (sugerencia: `feat(frontend): panel kyc de ine`,
  `feat(frontend): score por recibos y badges`).
- Commitea SOLO tus archivos poseídos (incluye el CSS generado).
- BRANCH ISOLATION (obligatorio): commitea y pushea SOLO a tu rama de
  worktree — `git push origin <tu-rama>` — tras cada commit. Nunca a `main`
  ni a otra rama; nunca merge/rebase/fast-forward de ramas ajenas.

## Report back

- Archivos creados/cambiados, salida del verify, decisiones de UI (ubicación
  de los paneles, textos), y cualquier fricción con la lectura de archivos en
  WASM (para el decision log).