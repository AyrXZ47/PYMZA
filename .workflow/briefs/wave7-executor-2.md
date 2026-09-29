# Brief: Wave 7 · Executor 2 (frontend)

> Copy of the planner's handoff. You never touch a file you don't own, even
> "obviously". Deviations go back to the planner via the decision log in
> `.workflow/plan.md`. Read `.workflow/plan.md` §"Ola 7 (actual)" COMPLETE before
> writing a single line: this brief is a summary, the plan is the source of truth.

## Task

Frontend de la ola 7, contra el contrato del executor-1:

1. **Cartera usable** (`components/cartera.rs`): buscador único por nombre /
   CURP / `_id` de plan, filtro por estado y producto (en memoria — los planes de
   una PYME son pocos miles), **dos tablas** (Activos incluye Moroso; Liquidados /
   inactivos debajo) cada una con su búsqueda/filtro y orden por columna (fecha,
   monto, saldo), columnas nuevas de **nombre del cliente**, **saldo**, **X/Y
   cubiertas**, y botón **Registrar abono** por plan, independiente de "Registrar
   pago": form inline con monto (default = saldo, editable) y nota opcional.
2. **Plan a 1 mes** (`components/plan_modal.rs`): opción "1 mes — Tasa 7%" en el
   select y tasas nuevas (`3→9%`, `6→12%`, `9→15%`, `12→18%`).
3. **api.rs**: `registrar_abono(plan_id, monto, nota, token)` + parsing de
   `saldo`/`cobrado`/`nombre` (helpers puros y testeados donde aplique, como
   `siguiente_cuota_impaga`).

No inventes endpoints ni campos: el contrato es el de `.workflow/plan.md`. Si algo
no cuadra con el backend, PARA y repórtalo.

## Definition of done

- Buscador y filtros funcionan sobre los datos ya cargados; la lista se puede
  vaciar y volver sin perder el filtro activo durante la sesión de la pantalla.
- Dos tablas separadas: Activos+Moroso arriba, Liquidados/inactivos abajo; si una
  está vacía, muestra su propio estado vacío (nunca una tabla fantasma).
- Botón "Registrar abono" abre su form inline (monto precargado con el `saldo`),
  llama `registrar_abono`, y al éxito refresca la cartera (saldo/estado al día);
  los errores del backend (400/404) se muestran en pantalla, no en consola.
- "Registrar pago" (cuota) sigue funcionando igual que hoy.
- El modal ofrece 1 mes con 7% y las tasas nuevas; el resto del flujo
  evaluar→autorizar→descargar contrato no cambia.
- Si agregas clases Tailwind nuevas, regenera el CSS con `./tailwind.sh` y
  commitea `assets/tailwind.css`.
- The verify command below passes.

## Files you own

- `frontend/src/components/cartera.rs`
- `frontend/src/components/plan_modal.rs`
- `frontend/src/api.rs`
- `frontend/tailwind.css` (input, si hiciera falta)
- `frontend/assets/tailwind.css` (compilado, SIEMPRE vía `./tailwind.sh`, nunca a mano)

## Files forbidden

- TODO `backend/**` (executor-1).
- `frontend/src/main.rs`, `frontend/src/components/dashboard.rs`,
  `frontend/src/components/charts.rs`, `frontend/src/components/alta_cliente.rs`,
  `frontend/src/components/sidebar.rs`, `frontend/src/components/landing.rs`,
  `login.rs`, `registro.rs` (el tablero y las novedades son ola 8).
- `frontend/Cargo.toml`, `frontend/Dioxus.toml`, `Dioxus.toml`, `AGENTS.md`.
- `.env*`, `.workflow/**`, `skills/**`, `PYMZA.md`, `docs/**`, `Dockerfile.*`,
  `docker-compose.yml`.
- La DB/API de producción: tu humo es local (`API_BASE` default
  `http://127.0.0.1:3000`).

## Read first

- `.workflow/plan.md` → §"Ola 7 (actual)": contrato API y semántica de saldo.
- `frontend/AGENTS.md`: referencia obligatoria de Dioxus 0.7 (signals, `rsx!`,
  `#[component]`, `spawn`). NO `cx`/`Scope`/`use_state`.
- `frontend/src/components/cartera.rs`: estructura actual (tabla + `FilaPlan` +
  mini-form inline de pago; ya usa `descargar_contrato` y `siguiente_cuota_impaga`).
- `frontend/src/api.rs`: `authed_request`, `sesion_ok`, `registrar_pago`,
  `parsear_resumen`, patrón de helpers puros + tests.
- `frontend/src/components/plan_modal.rs`: select de plazos y flujo evaluar/autorizar.
- `frontend/clippy.toml`: nada de signals vivos sobre un await (lee el token
  ANTES del `await`).

## Verify command

```bash
cd frontend && cargo check --target wasm32-unknown-unknown && cargo test && ./tailwind.sh
```

## Commit

- MANDATORY: conventional commits, short summary, imperative, one line
  (`feat:`, `fix:`, `docs:`, `refactor:`, `test:`), <72 chars, no AI attribution,
  no trailers.
- Commits lógicos separados (p. ej. `feat(frontend): buscador y filtros de
  cartera` / `feat(frontend): registro de abonos` / `feat(frontend): plazo de 1
  mes en el plan de pagos`). NUNCA mezcles el CSS regenerado con otra cosa.
- Commit ONLY your owned files. Si crees que otro archivo necesita cambio,
  repórtalo, no lo toques.
- BRANCH ISOLATION (mandatory): `git push origin wave7-executor-2` después de cada
  commit. Nunca a `main` ni a otra rama; nunca merge/rebase/checkout.

## Report back

- Archivos cambiados, salida del verify command, decisiones de UI (p. ej. qué
  columnas ordenables elegiste), cualquier desviación del contrato del backend, y
  preguntas abiertas.
