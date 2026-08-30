# Brief: Wave 3 · Executor 2 — Frontend: verificación de teléfono en el alta de cliente

> Copia de `.workflow/briefs/_template.md`. El executor no toca archivos que no
> posee, ni "obviamente". Desviaciones → decision log en `.workflow/plan.md`.

## Tarea

Añadir el flujo de verificación de teléfono por OTP al alta de cliente,
implementando el "Contrato API ola 3" de `.workflow/plan.md`:

1. **`api.rs`**: dos helpers públicos nuevos que siguen el patrón existente
   (`authed_request` + `sesion_ok`):
   - `solicitar_verificacion(curp, telefono, token)` → `POST
     /api/verificaciones/solicitar` con body `{ curp, telefono }`.
   - `confirmar_verificacion(curp, telefono, codigo, token)` → `POST
     /api/verificaciones/confirmar` con body `{ curp, telefono, codigo }`.
   Manejar 401 con `sesion_ok` (como el resto de llamadas autenticadas).
2. **`alta_cliente.rs`**:
   - El alta de cliente envía `correo` opcional si el usuario lo capturó
     (campo nuevo en el form, opcional; si está vacío, no mandarlo o mandar
     `null`).
   - Tras un alta exitosa (o al ver un cliente existente ya cargado en el
     panel), si `cliente.telefono_verificado` es `false`, mostrar la sección
     **"Verificar teléfono"**: teléfono (prellenado del cliente), botón
     "Enviar código" → input de 6 dígitos → botón "Confirmar". Estados:
     enviando, código enviado (mensaje "Revisa tu WhatsApp — en dev el código
     aparece en el log del backend"), confirmando, verificado (badge "✓
     Verificado" verde), error (código inválido/expirado → mensaje).
   - Si `telefono_verificado` es `true`, mostrar el badge directamente (tanto
     en el alta como en el resultado de búsqueda por CURP).
   - Ojo: el modal de plan de pagos (`plan_modal.rs`) NO cambia; el badge
     solo aparece en el panel de validación/alta.
3. Tests unitarios en host (no wasm) para la lógica pura que añadas:
   - parseo del estado de verificación de la respuesta JSON,
   - construcción del body de solicitar/confirmar,
   - al menos 2 casos; mantén los 11 tests existentes pasando.
   Sin clases Tailwind nuevas SI es posible (reusa los estilos existentes del
   form: `bg-slate-900 dark:bg-slate-800` etc.); si necesitas una clase
   nueva, regenérate el CSS con `./tailwind.sh` y commitea
   `frontend/assets/tailwind.css`.

Ponytail: sin componente nuevo de "pasos wizard" — la sección de verificación
es un bloque condicional dentro del panel existente. Sin dependencias nuevas.

## Definition of done

- Form de alta de cliente incluye correo opcional y lo envía al backend.
- Cliente no verificado → sección "Verificar teléfono" visible con los 3
  estados (enviar → código → confirmar); cliente verificado → badge.
- Búsqueda por CURP muestra el badge de verificado cuando
  `telefono_verificado: true`.
- Errores (código inválido, 401) se muestran al usuario sin panickear.
- 11 tests existentes + los nuevos pasan. El comando de verify de abajo pasa.

## Files you own

- `frontend/src/**`
- `frontend/tailwind.css` (solo si añades clases nuevas)
- `frontend/assets/tailwind.css` (generado; solo si cambiaste clases)

## Files forbidden

- `backend/**`, `AGENTS.md` (raíz), `README.md`, `docs/**`
- `frontend/tailwind.sh`, `frontend/Dioxus.toml`, `frontend/AGENTS.md`,
  `frontend/Cargo.toml` (cero deps nuevas)
- `PYMZA.md`, `docker-compose.yml`, `Dockerfile.*`, `.workflow/**`, `skills/**`

## Read first

- `.workflow/plan.md` — sección "Ola 3 (actual)" y su Contrato API.
- `frontend/src/api.rs` (helpers y patrón `authed_request`/`sesion_ok`).
- `frontend/src/components/alta_cliente.rs` (form actual, estados, estilos).
- `frontend/src/components/login.rs` (patrón de señales para flujos con
  estados: error_msg, éxito, deshabilitar botones mientras se envía).

## Verify command

```bash
cd frontend && cargo check --target wasm32-unknown-unknown && cargo test
```

(Si añadiste clases Tailwind: añade `&& ./tailwind.sh` y commitea el CSS.)

## Commit

- MANDATORY: conventional commits, resumen corto, imperativo, una línea
  (`feat:`, `fix:`, `chore:`, `docs:`, `refactor:`, `test:`, `perf:`,
  `style:`, `build:`, `ci:`, `revert:`, scope opcional). <72 chars. Sin
  atribución de IA, sin trailers.
- Un cambio lógico por commit (sugerencia: `feat(frontend): verificacion de
  telefono por otp en alta de cliente`).
- Commitea SOLO tus archivos poseídos (incluye el CSS generado si cambió).
- BRANCH ISOLATION (obligatorio): commitea y pushea SOLO a tu rama de
  worktree — `git push origin <tu-rama>` — tras cada commit. Nunca a `main`
  ni a otra rama; nunca merge/rebase/fast-forward de ramas ajenas.

## Report back

- Archivos creados/cambiados, salida del verify, captura de los estados del
  flujo (enviado/confirmado/error), y cualquier decisión de UI tomada
  (colores del badge, textos).