# Brief: Wave 1 · Executor 2 — Frontend: módulos + contrato API nuevo + sesión persistente

> Copia de `.workflow/briefs/_template.md`. El executor no toca archivos que no
> posee, ni "obviamente". Desviaciones → decision log en `.workflow/plan.md`.

## Tarea

Partir el monolito `frontend/src/main.rs` (~1025 líneas) en módulos y adaptar
toda la app al "Contrato API ola 1" de `.workflow/plan.md`. En concreto:

1. **Split (sin cambio de comportamiento ni de estilos):**
   - `frontend/src/main.rs`: solo `main()`, `App` y wiring (<200 líneas).
   - `frontend/src/api.rs`: `API_BASE`, `http_client()`, `authed_request()`,
     `sesion_ok()` y helpers compartidos.
   - `frontend/src/components/`: un módulo por pantalla/componente
     (`login.rs`, `sidebar.rs`, `dashboard.rs`, `alta_cliente.rs`,
     `cartera.rs`, y el modal de plan de pagos). Sin router (seguimos con
     `MenuState`).
2. **Adaptación al contrato (backend cambia en paralelo; codifica contra el
   contrato del plan, no contra el backend actual):**
   - `GET /api/creditos` y `GET /api/dashboard` — sin `/{empresa}` en la URL.
   - `POST /api/creditos/autorizar` y `POST /api/clientes/:curp/reportar` —
     quitar `empresa` del body JSON.
   - Eliminar TODO uso hardcodeado de `token-temporal-123` (hay uno en un
     test y otro en la lógica de arranque, ~línea 300): el arranque valida la
     sesión llamando a `/api/dashboard` con el token guardado; si 401,
     logout.
3. **Sesión persistente (localStorage):** al login/registro exitoso guardar el
   token en `localStorage`; al montar `App`, leerlo y revalidar; en logout,
   borrarlo. Es la migración que el comentario `ponytail:` de `authed_request`
   ya anuncia. Usa `use_effect` (localStorage solo existe tras hidratación).
   Si localStorage falla/disponible no, fallback silencioso a sesión en
   memoria.
4. Actualiza los tests existentes (`#[cfg(test)]` en main.rs) al contrato
   nuevo; añade test del helper si la lógica de sesión lo amerita.

CERO clases Tailwind nuevas (no se regenera CSS). CERO dependencias nuevas.
Ojo con `frontend/clippy.toml`: las señales se copian ANTES del `await`
(`let token_val = token();`) — conserva ese patrón en cada spawn.

## Definition of done

- `frontend/src/main.rs` <200 líneas; componentes en módulos separados; la UI
  y los textos idénticos a antes (mismo comportamiento, misma estética).
- Ninguna URL contiene `/{empresa}` para creditos/dashboard; ningún body
  envía `empresa`; `rg "token-temporal-123" frontend/` → 0 hits.
- Token persiste en localStorage entre recargas; logout lo borra.
- `cargo check --target wasm32-unknown-unknown` y `cargo test` pasan.
- El comando de verify de abajo pasa.

## Files you own

- `frontend/src/**`
- `frontend/Cargo.toml` (solo si el split lo exige; PROHIBIDO añadir deps)

## Files forbidden

- `backend/**` (el executor-1 lo cambia en paralelo; tú codificas contra el
  contrato del plan)
- `frontend/tailwind.css`, `frontend/tailwind.sh`, `frontend/Dioxus.toml`,
  `frontend/assets/**` (nada de estilos ni config en esta ola)
- `README.md`, `docs/**`, `PYMZA.md`, `AGENTS.md`, `.workflow/**`, `skills/**`

## Read first

- `frontend/AGENTS.md` — referencia obligatoria de Dioxus 0.7 (señas, no
  `cx`/`use_state`, `use_effect` para localStorage).
- `frontend/src/main.rs` completo — es TODO el estado actual.
- `.workflow/plan.md` — sección **Contrato API ola 1** (tu fuente de verdad).
- `frontend/clippy.toml` — el lint de señales sobre `await`.

## Verify command

```bash
cd frontend && cargo check --target wasm32-unknown-unknown && cargo test
```

## Commit

- MANDATORY: conventional commits, resumen corto, imperativo, una línea
  (`feat:`, `fix:`, `chore:`, `docs:`, `refactor:`, `test:`, `perf:`,
  `style:`, `build:`, `ci:`, `revert:`, scope opcional). <72 chars. Sin
  atribución de IA, sin trailers.
- Un cambio lógico por commit (el split y la adaptación al contrato pueden
  ser commits separados dentro de tu rama).
- Commitea SOLO tus archivos poseídos.
- BRANCH ISOLATION (obligatorio): commitea y pushea SOLO a tu rama de
  worktree — `git push origin <tu-rama>` — tras cada commit. Nunca a `main`
  ni a otra rama; nunca merge/rebase/fast-forward de ramas ajenas.
- Sugerencia: `refactor(frontend): monolito a modulos` y
  `feat(frontend): contrato api ola 1 y sesion persistente`.

## Report back

- Archivos creados/cambiados, salida del verify, líneas finales de
  `main.rs`, y cualquier punto donde el contrato del plan resultara ambiguo
  (para el decision log).
