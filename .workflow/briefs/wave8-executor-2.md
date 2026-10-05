# Brief: Wave 8 · Executor 2 (frontend)

> Copy of the planner's handoff. You never touch a file you don't own, even
> "obviously". Deviations go back to the planner via the decision log in
> `.workflow/plan.md`. Read `.workflow/plan.md` §"Ola 8 (actual)" COMPLETE before
> writing a single line: this brief is a summary, the plan is the source of truth.

## Task

Frontend de la ola 8, contra el contrato del executor-1:

1. **Dashboard honesto** (`components/dashboard.rs` + `components/charts.rs`):
   tarjetas de KPI con **selector de periodo** (semana / mes / bimestre /
   trimestre / semestre) que se traduce a `?desde=YYYY-MM-DD&hasta=YYYY-MM-DD` y
   refetchea dashboard + resumen; muestra `capital_colocado`, `cobrado_periodo`,
   `por_cobrar_neto`, `cartera_vencida` y `tasa_morosidad` (dinero). Corrige la
   semántica de las gráficas para que consuman el resumen recalculado (sin
   estados rancios) y ajusta etiquetas/leyendas a dinero.
2. **Campanita de novedades** (`components/novedades.rs` nuevo, montada en
   `components/sidebar.rs`): const `APP_VERSION` compilada vs `GET /api/novedades`;
   - versión servidor > `APP_VERSION` → banner **"Hay una actualización
     disponible — recarga la página"**;
   - versiones no vistas (`localStorage pymza_novedades_v`) → badge; al abrir, modal
     con el changelog y marca como vistas.
3. **api.rs**: `obtener_dashboard_periodo`, `obtener_novedades`, `APP_VERSION`,
   helpers de periodo (`rango_periodo(preset, hoy)` puro y testeado) y parsing.

No inventes endpoints ni campos: el contrato es `.workflow/plan.md` §Ola 8. Si algo
no cuadra con el backend, PARA y repórtalo.

## Definition of done

- Selector de periodo funcional (los 5 presets) que cambia KPIs y la serie
  cobrado-vs-por-cobrar; el preset por defecto es "mes" y no rompe si no hay datos.
- KPIs visibles y coherentes: colocado / cobrado (periodo) / por cobrar neto /
  vencido / morosidad, con formato de moneda consistente.
- Gráficas corregidas: ya no muestran `Activo` rancio (el backend manda el estado
  recalculado); etiquetas claras de qué es dinero y qué es conteo.
- Campanita: con `version` del servidor mayor que `APP_VERSION` aparece el banner
  de recarga; con novedades nuevas aparece el badge; al abrir se ve el changelog y
  el badge desaparece (persistido en localStorage). Sin errores si `/api/novedades`
  falla (la app no se rompe).
- Si agregas clases Tailwind nuevas, regenera el CSS con `./tailwind.sh` y
  commitea `assets/tailwind.css`.
- The verify command below passes.

## Files you own

- `frontend/src/components/dashboard.rs`
- `frontend/src/components/charts.rs`
- `frontend/src/components/sidebar.rs`
- `frontend/src/components/novedades.rs` (NUEVO)
- `frontend/src/api.rs`
- `frontend/src/main.rs` (montar novedades si hace falta)
- `frontend/tailwind.css` (input, si hiciera falta)
- `frontend/assets/tailwind.css` (compilado, SIEMPRE vía `./tailwind.sh`)

## Files forbidden

- TODO `backend/**` (executor-1).
- `frontend/src/components/cartera.rs`, `frontend/src/components/plan_modal.rs`,
  `frontend/src/components/alta_cliente.rs` (son de la ola 7 / ola 9).
- `frontend/Cargo.toml` (cero deps nuevas), `frontend/Dioxus.toml`, `AGENTS.md`.
- `.env*`, `.workflow/**`, `skills/**`, `PYMZA.md`, `docs/**`, `Dockerfile.*`,
  `docker-compose.yml`.
- La API/DB de producción: tu humo es local (`API_BASE` default
  `http://127.0.0.1:3000`).

## Read first

- `.workflow/plan.md` §"Ola 8 (actual)": contrato API y shape de KPIs/novedades.
- `.workflow/audits/wave7.md` §Observaciones (O1): el motivo del "tablero rancio".
- `frontend/AGENTS.md`: referencia obligatoria de Dioxus 0.7.
- `frontend/src/components/dashboard.rs`: cards actuales y `use_resource` del
  resumen; patrón de fetch con `authed_request`/`sesion_ok`.
- `frontend/src/components/charts.rs`: `BarraApilada`, `Linea`, `Donut`, `BarraH`,
  `semaforo_morosidad` y sus tests.
- `frontend/src/api.rs`: `obtener_resumen`, `Resumen`, patrón de consts
  (`API_BASE`, `TOKEN_STORAGE_KEY`) y helpers puros testeados.
- `frontend/clippy.toml`: nada de signals vivos sobre un await.

## Verify command

```bash
cd frontend && cargo check --target wasm32-unknown-unknown && cargo test && ./tailwind.sh
```

## Commit

- MANDATORY: conventional commits, short summary, imperative, one line, <72
  chars, no AI attribution, no trailers.
- Commits lógicos separados: `feat(frontend): filtros de periodo en dashboard`,
  `fix(frontend): graficas con estado recalculado`, `feat(frontend): campanita de
  novedades`. El CSS regenerado va en su propio commit.
- Commit ONLY your owned files. Reporta cualquier otro archivo que creas que
  necesita cambio, no lo toques.
- BRANCH ISOLATION (mandatory): `git push origin wave8-executor-2` después de cada
  commit. Nunca a `main` ni a otra rama; nunca merge/rebase/checkout.

## Report back

- Archivos cambiados, salida del verify command, decisiones de UI (presets,
  formato de moneda, dónde vive la campanita), desviaciones del contrato y
  preguntas.
