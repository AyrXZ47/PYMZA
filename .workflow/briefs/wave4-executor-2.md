# Brief: Wave 4 · Executor 2 — Frontend: gráficas SVG en dashboard, registrar pago en cartera, favicon

> Copia de `.workflow/briefs/_template.md`. El executor no toca archivos que no
> posee, ni "obviamente". Desviaciones → decision log en `.workflow/plan.md`.

## Tarea

Implementar la parte frontend del "Contrato API ola 4" de
`.workflow/plan.md`:

1. **`components/charts.rs`** (nuevo): primitivas de gráficas en SVG puro
   dentro de rsx (rect/path/circle/text/polyline) — CERO librería de charts,
   CERO dependencias. Cuatro primitivas:
   - `BarraApilada` (2 series, p.ej. cobrado vs por cobrar por mes).
   - `Linea` (puntos + polyline, p.ej. flujo proyectado).
   - `Donut` (segmentos con stroke-dasharray, p.ej. distribución de montos).
   - `BarraH` (barras horizontales con etiqueta y valor, p.ej. top deudores,
     aging).
   Todas con leyenda/etiquetas legibles y colores Tailwind con variantes
   dark: (blue-500, emerald-500, amber-500, red-500, slate). Pon un techo
   `ponytail:` en charts.rs: "sin interactividad (tooltips/zoom); si se
   necesita, migrar a librería".
2. **`dashboard.rs`**: debajo de los 3 KPIs actuales ("Créditos Activos",
   "Capital Prestado", "Próximos Cobros"), grid de 2 columnas con las 6
   gráficas del resumen (cargado de `GET /api/creditos/resumen` con
   `use_resource`, estados loading/error/vacío — si no hay datos, mostrar
   "Sin datos aún" dentro del card, no un error):
   - Cobrado vs por cobrar (mes actual + 5 previos) — `BarraApilada`.
   - Tasa de morosidad — KPI semáforo (% con color: <5% verde, 5-20% ámbar,
     >20% rojo) + número grande.
   - Flujo de caja proyectado (30/60/90 días) — `Linea`.
   - Distribución de créditos por monto — `Donut`.
   - Aging de cartera (0-30/31-60/61-90/90+) — `BarraH`.
   - Top 10 clientes con mayor deuda — `BarraH`.
   (Las demás de la captura de V están DIFERIDAS por falta de datos — plan,
   decision log 2026-08-31. No inventar datos.)
3. **`cartera.rs`**: para cada plan, badge de estado (`Activo` verde,
   `Moroso` ámbar, `Liquidado` gris/slate) + "Cuota X/Y pagadas" + botón
   **"Registrar pago"** que despliega un mini-form inline (cuota con select
   de la siguiente cuota impaga, monto prellenado con `pago_mensual`) →
   `POST /api/creditos/pagos` → al éxito refrescar la lista y el badge.
   Errores (duplicado, monto inválido) visibles al usuario.
4. **`api.rs`**: helpers `obtener_resumen(token)` y `registrar_pago(token,
   plan_id, cuota, monto)` siguiendo el patrón `authed_request`/`sesion_ok`,
   + structs de parseo del resumen (serde). El `_id` del plan llega como
   string hex en la respuesta — no assumes otra forma.
5. **Favicon y título** (pedido explícito de V):
   - `frontend/assets/favicon.svg` (nuevo): logo simple de PYMZA — monograma
     "P" (o símbolo abstracto de crédito/cobranza) con la paleta de la marca
     (azul sobre fondo oscuro). SVG a mano, <1KB.
   - `main.rs`: `document::Link { rel: "icon", href: asset!("/assets/favicon.svg") }`.
   - `Dioxus.toml`: `title = "PYMZA — Crédito con cobranza respaldada"`
     (única edición permitida en este archivo).
6. Tests en host para la lógica pura nueva: parseo del resumen (shape del
   contrato), semáforo de morosidad (<5/5-20/>20), selección de siguiente
   cuota impaga. Los 11 tests existentes deben seguir pasando.
7. Si añades clases Tailwind nuevas (probable): regenerar con
   `./tailwind.sh` y commitear `frontend/assets/tailwind.css`.

Ponytail: primitivas planas, sin framework de gráficas ni abstracciones de
estado; el grid es layout Tailwind estándar.

## Definition of done

- Bajo los 3 KPIs se ven las 6 gráficas con datos reales del resumen (o
  "Sin datos aún" si vacío).
- Cartera muestra badge de estado y cuotas pagadas; "Registrar pago" funciona
  end-to-end (prellenado, éxito refresca, errores visibles).
- La pestaña del navegador muestra favicon y título PYMZA.
- Cero deps nuevas; `cargo check --target wasm32-unknown-unknown` y
  `cargo test` pasan; el comando de verify de abajo pasa.

## Files you own

- `frontend/src/**`
- `frontend/Dioxus.toml` (SOLO la key `title`)
- `frontend/tailwind.css` (solo si añades clases nuevas)
- `frontend/assets/**` (favicon.svg + tailwind.css regenerado)

## Files forbidden

- `backend/**`, `AGENTS.md` (raíz), `README.md`, `docs/**`
- `frontend/tailwind.sh`, `frontend/AGENTS.md`, `frontend/Cargo.toml`
  (cero deps nuevas), cualquier otra key de `Dioxus.toml`
- `PYMZA.md`, `docker-compose.yml`, `Dockerfile.*`, `.workflow/**`, `skills/**`

## Read first

- `.workflow/plan.md` — sección "Ola 4 (actual)" y Contrato API (shape exacta
  del resumen; es tu fuente para los structs serde).
- `frontend/src/components/dashboard.rs` (KPIs actuales y estilos),
  `frontend/src/components/cartera.rs` (lista de planes, patrón de refresh),
  `frontend/src/api.rs` (helpers y sesión), `frontend/src/main.rs` (App).
- `frontend/AGENTS.md` — referencia Dioxus 0.7 (`use_resource` para datos,
  señales antes de `await`).

## Verify command

```bash
cd frontend && cargo check --target wasm32-unknown-unknown && cargo test && ./tailwind.sh && git diff --stat assets/tailwind.css
```

(El último comando muestra el CSS regenerado — evidencia si cambiaste clases.)

## Commit

- MANDATORY: conventional commits, resumen corto, imperativo, una línea
  (`feat:`, `fix:`, `chore:`, `docs:`, `refactor:`, `test:`, `perf:`,
  `style:`, `build:`, `ci:`, `revert:`, scope opcional). <72 chars. Sin
  atribución de IA, sin trailers.
- Un cambio lógico por commit (sugerencia: `feat(frontend): primitivas svg de
  graficas`, `feat(frontend): dashboard con graficas de impacto`,
  `feat(frontend): registrar pago en cartera`, `feat(frontend): favicon y
  titulo pymza`).
- Commitea SOLO tus archivos poseídos (incluye el CSS generado).
- BRANCH ISOLATION (obligatorio): commitea y pushea SOLO a tu rama de
  worktree — `git push origin <tu-rama>` — tras cada commit. Nunca a `main`
  ni a otra rama; nunca merge/rebase/fast-forward de ramas ajenas.

## Report back

- Archivos creados/cambiados, salida del verify, decisiones de UI (paleta de
  cada gráfica, textos), y cualquier punto donde el shape del resumen no
  cupiera en una primitiva (para el decision log).