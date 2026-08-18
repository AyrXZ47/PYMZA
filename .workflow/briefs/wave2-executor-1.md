# Brief: Wave 2 · Executor 1 — Frontend: portal público (landing + registro/login + tema + API_BASE)

> Copia de `.workflow/briefs/_template.md`. El executor no toca archivos que no
> posee, ni "obviamente". Desviaciones → decision log en `.workflow/plan.md`.

## Tarea

Convertir el arranque del frontend en un portal que vende, implementando
EXACTAMENTE el "Comportamiento esperado" de la ola 2 en `.workflow/plan.md`:

1. **`main.rs`** (102 líneas hoy): añadir `enum VistaPublica { Landing, Login,
   Registro }` y un signal `vista_publica` en `App`. Sin auth → render de la
   vista pública actual; con auth → `Sidebar` + `MainArea` (como hoy). Sin
   router (ponytail: ver decision log 2026-08-17).
2. **`components/landing.rs` (nuevo)**: página pública de venta — hero con el
   pitch, sección de beneficios (score con datos alternativos, red de alerta
   temprana, planes de pago estructurados, cartera + dashboard), CTAs "Crear
   cuenta" (→ `VistaPublica::Registro`) e "Iniciar sesión" (→
   `VistaPublica::Login`). Toggle de tema también aquí.
3. **`components/registro.rs` (nuevo)**: mover el form de alta de empresa
   desde `login.rs` (hoy está embebido al fondo del Login; `reg_*` signals y
   handler). Al éxito de `POST /api/empresas` → **auto-login**: `POST
   /api/login` con las mismas credenciales → `is_authenticated=true`, token
   guardado (misma clave localStorage que `api.rs`), `current_company` set. Si
   el auto-login fallara (raro), mostrar mensaje con botón "Ir a Iniciar
   sesión". Enlace "¿Ya tienes cuenta? Inicia sesión" → `VistaPublica::Login`.
4. **`components/login.rs`**: quedarse solo con correo+password+entrar:
   eliminar la sección "Registrar Empresa" y sus signals. Enlace "¿No tienes
   cuenta? Regístrate" → `VistaPublica::Registro`. Enlace/logo opcional → Landing.
5. **Tema claro/oscuro**:
   - `tailwind.css`: añadir `@custom-variant dark (&:where(.dark, .dark *));`
     al inicio (Tailwind v4).
   - Migrar TODOS los componentes (`login`, `sidebar`, `dashboard`, `cartera`,
     `alta_cliente`, `plan_modal`, y los nuevos) a pares base-light + `dark:`
     manteniendo el look oscuro actual cuando `dark` está en `<html>`.
   - App: al arrancar leer `localStorage["pymza_theme"]` (default "dark");
     aplicar/remover clase `dark` en `document.documentElement` (usa
     `document::eval` de Dioxus 0.7 — ver `frontend/AGENTS.md`). Toggle 🌙/☀️
     que invierte y persiste la preferencia.
   - Regenerar el CSS con `./tailwind.sh` y **commitear
     `frontend/assets/tailwind.css`** (está commiteado a propósito en este
     repo; el `dx` de nixpkgs no lo compila solo).
6. **`api.rs`**: `pub const API_BASE: &str =
   option_env!("API_BASE").unwrap_or("http://127.0.0.1:3000");` — configurable
   en build/deploy; default dev sin cambios. Ajustar el test existente si
   construye URLs (debe seguir pasando con el default).
7. Tests: los 8 existentes deben seguir pasando; añade tests puros para la
   lógica del tema que sea comprobable en host (no wasm).

Ponytail: cero dependencias nuevas, cero router, cero abstracciones de theme
(no framework de estados); clase `dark` + localStorage es la solución más corta
que funciona. Marca con `ponytail:` los atajos deliberados nombrando su techo.

## Definition of done

- Sin auth, la app muestra la Landing (no login) al abrir.
- "Crear cuenta" lleva a Registro; registrar una empresa nueva → entra a la app
  autenticada (auto-login verificado con network en vivo o test).
- Login ya no contiene el form de registro embebido; enlaces cruzados
  Login ↔ Registro funcionan.
- Toggle de tema cambia la paleta (dark por defecto), persiste al recargar.
- `API_BASE` usa `option_env!` con default dev.
- `./tailwind.sh` regenera `assets/tailwind.css` y el archivo queda commiteado.
- Build y tests: el comando de verify de abajo pasa.

## Files you own

- `frontend/src/**`
- `frontend/tailwind.css`
- `frontend/assets/tailwind.css` (generado; se commitea)

## Files forbidden

- `backend/**`, `AGENTS.md` (raíz), `README.md`, `docs/**`
- `frontend/tailwind.sh`, `frontend/Dioxus.toml`, `frontend/AGENTS.md`,
  `frontend/Cargo.toml` (cero deps nuevas)
- `PYMZA.md`, `docker-compose.yml`, `Dockerfile.*`, `.workflow/**`, `skills/**`

## Read first

- `.workflow/plan.md` — sección "Ola 2 (actual)" (tu fuente de verdad).
- `frontend/AGENTS.md` — referencia obligatoria de Dioxus 0.7 (`document::eval`
  para manipular `<html>`, signals, `spawn`, `use_effect`).
- `frontend/src/main.rs` + `frontend/src/components/login.rs` (el form a
  mover) + `frontend/src/api.rs` (constante API_BASE y sesión).

## Verify command

```bash
cd frontend && cargo check --target wasm32-unknown-unknown && cargo test && ./tailwind.sh && git diff --stat assets/tailwind.css
```

(El último comando debe mostrar el CSS regenerado — evidencia del tema.)

## Commit

- MANDATORY: conventional commits, resumen corto, imperativo, una línea
  (`feat:`, `fix:`, `chore:`, `docs:`, `refactor:`, `test:`, `perf:`,
  `style:`, `build:`, `ci:`, `revert:`, scope opcional). <72 chars. Sin
  atribución de IA, sin trailers.
- Un cambio lógico por commit. Uno o más commits por tarea (sugerencia:
  `feat(frontend): landing y registro con auto-login`,
  `feat(frontend): tema claro/oscuro`, `build(frontend): regenerar css dark`).
- Commitea SOLO tus archivos poseídos (incluye el CSS generado).
- BRANCH ISOLATION (obligatorio): commitea y pushea SOLO a tu rama de
  worktree — `git push origin <tu-rama>` — tras cada commit. Nunca a `main`
  ni a otra rama; nunca merge/rebase/fast-forward de ramas ajenas.

## Report back

- Archivos creados/cambiados, salida del verify, decisiones de UI que hayas
  tomado al migrar colores (mapeo light/dark aplicado), desviaciones del plan
  (si las hubo y por qué).