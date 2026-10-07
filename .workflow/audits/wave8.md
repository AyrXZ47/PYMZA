# Auditoría Ola 8 — Tablero honesto, novedades y cierre E1/E2

- **Fecha:** 2026-10-07 (sesión de auditoría en fresco, árbol integrado `main` @ `84079a6`)
- **Alcance:** `.workflow/plan.md` §"Ola 8 (actual)", briefs `wave8-executor-{1,2}.md`, `.workflow/audit-checklist.md`
- **Release gate:** `skills/security-audit` (6 fases, 2 hunters independientes + validación propia) — **cero CRITICAL/HIGH**; 2 MEDIUM de integridad diferidos como excepciones con owner.
- **Veredicto: APPROVED WITH EXCEPTIONS** (E1, E2 MEDIUM; O1 LOW — no bloquean el release gate; owner planner)

> Nota de entorno (no es hallazgo de código): el `rustup` gestionado por Nix tiene un
> wrapper de linker roto (`ld.lld` apunta a `/nix/store/…-rustup-1.29.0/nix-support/ld-wrapper.sh`,
> ya inexistente; el store tiene 1.29.1). Todo build/test de esta auditoría corrió con
> `RUSTFLAGS="-C link-arg=-fuse-ld=bfd"`. Sin ese flag, `cargo build`/`cargo test` fallan
> al linkear. Acción para V: re-purga/actualiza el toolchain rustup de NixOS (el fix no es de este repo).

---

## 1. Integridad de la integración

Base de la ola: `88398ba` ("docs(plan): ola 7 auditada; ola 8 con E1/E2 + briefs").

| Check | Evidencia | OK |
|---|---|---|
| Worktrees mergeados a main | `git log --oneline 88398ba..wave8-executor-1` → `dcd28e2, 413d697, cb0d738, 7dc9f52, 1e3d87d`; `88398ba..wave8-executor-2` → `ae91df5, d6a0882, a930fb4, d86905e`. Merges `0b8e73c` (e1) y `84079a6` (e2); head integrado `84079a6`. Ambas ramas contenidas en `main` | ✅ |
| `git status` limpio, sin stashes | `git status --porcelain` → sin salida; `git stash list` → vacío; `main == origin/main` (0/0) | ✅ |
| Nada fuera del mapa de propiedad | `git diff --name-status 88398ba main` → 12 archivos: backend `{main,models/credito,routes/credito,novedades}.rs` + `docs/API.md` (e1); frontend `{api,components/{dashboard,charts,sidebar,novedades,mod}}.rs` + `assets/tailwind.css` (e2). **Único fuera del mapa: `frontend/src/components/mod.rs`** (+1 línea `pub mod novedades;`, wiring necesario para compilar) | ⚠️ menor |
| Todo lo planeado presente | E1 `reservar_cobrado`/reconciliación, E2 recompute, KPIs por ventana, `resumen_cartera` con estado recalculado, `GET /api/novedades`, dashboard, campanita, `docs/API.md` +138 | ✅ |
| Cero archivos fuera de los briefs | `git log --no-merges 88398ba..wave8-executor-{1,2} --name-only` → solo los archivos del mapa (+`mod.rs`). `git log --stat` por rama sin cruces | ✅ |

**Merges `--stat`:** e1 = 5 commits; e2 = 4 commits. Diff total = 1486 inserciones / 167 borrados en 12 archivos; `git diff 88398ba main -- '*Cargo.toml'` → **vacío (cero deps nuevas)**.

## 2. Build y tests (árbol integrado)

| Comando | Salida | OK |
|---|---|---|
| `cd backend && MONGODB_URI=… cargo build` (con `-fuse-ld=bfd`) | `Finished dev profile` | ✅ |
| `cd backend && MONGODB_URI=… cargo test` | `87 passed (1 suite, 0 failed)` — ola 7 tenía 81 (+6) | ✅ |
| `cd frontend && cargo check --target wasm32-unknown-unknown` | `Finished dev profile` | ✅ |
| `cd frontend && cargo test` | `60 passed (1 suite, 0 failed)` — ola 7 tenía 48 (+12) | ✅ |
| `cd frontend && ./tailwind.sh` → CSS en sync | `tailwindcss v4.3.3 - Done in 39ms`; `git status --porcelain` → **sin salida** (CSS commiteado byte-idéntico) | ✅ |
| `cargo clippy --all-targets` (backend) y `cargo clippy --target wasm32…` (frontend) | 0 errores, 0 warnings | ✅ |

Tests nuevos relevantes: backend `pago_mensual_recalculado_ignora_el_body`, `rango_fechas_valida_ordena_o_none`, `stats_dashboard_kpis_en_vivo_y_ventana`, `resumen_serie_respeta_la_ventana`, `plan_json_recalcula_el_estado_persistido_rancio`, `novedades_shape_del_contrato`; frontend `rango_periodo_*`, `version_es_mayor_compara_segmentos_numericos`, `parsear_dashboard_*`, `parsear_novedades_*`, `fmt_moneda_agrupa_miles_y_dos_decimales`.

## 3. Verify de briefs + humo en vivo (DB local, mongod standalone)

Entorno: `mongod --dbpath ~/.mongo-data` (127.0.0.1:27017) + `mongosh < backend/scripts/seed.js`; backend `target/debug/pymza_backend` con **`MONGODB_URI=mongodb://127.0.0.1:27017`** forzado. Cero efectos en Atlas/producción.

| Check del contrato | Resultado | OK |
|---|---|---|
| **E1** plan deuda 3000 + 5 abonos concurrentes de 1000 | **exactamente 3×200 y 2×400**; `planes_pago.cobrado=3000`, 3 docs en `pagos`; API → `cobrado 3000, saldo 0, estado Liquidado, cuotas_pagadas 6` | ✅ |
| **E2** `autorizar {monto_total:100000, plazo_meses:6, pago_mensual:0.01, tasa_interes:0}` | persiste `pago_mensual 18666.67`, `tasa_interes 0.12` (= `tasa_por_plazo(6)`); deuda `112000.02`, **no 0.06** | ✅ |
| Regresión ola 7: abono parcial no marca cuota | deuda 3000, abono 200 → `cobrado 200, saldo 2800, cuotas_pagadas 0, Activo` | ✅ |
| Regresión ola 7: cuota duplicada serial | `pagos` cuota 1 duplicada → `400 "Cuota ya registrada"` | ✅ |
| Regresión ola 7: `Pago` legacy sin `tipo` | doc sin `tipo` leído como cuota → `cobrado 1210, cuotas_pagadas 2` | ✅ |
| O2: `nota` acotada | nota de 300 chars persistida con `len=280` | ✅ |
| **O1**: `estado` recalculado en resumen | plan persistido `Activo` con cuotas vencidas → API `Moroso, cuotas_vencidas 6`; `aging.90+ = 3000` en el resumen (no lo excluye el filtro persistido) | ✅ |
| Dashboard sin ventana | 5 KPIs nuevos + 3 viejos presentes | ✅ |
| `GET /api/dashboard?desde=hoy&hasta=hoy` | `cobrado_periodo 4210` == suma directa en Mongo (`$sum monto fecha=hoy` = 4210) | ✅ |
| `GET /api/dashboard?desde=2020-01-01&hasta=2020-12-31` | `cobrado_periodo 0` | ✅ |
| `tasa_morosidad` money-based | `cartera_vencida 6391.98 / capital_colocado 109200 = 0.05853` (coincide) | ✅ |
| `GET /api/creditos/resumen?desde&hasta` | ventana `hoy` → 1 mes (`2026-10`, cobrado 4210); ventana 2020 → 12 meses en 0; serie respeta la ventana | ✅ |
| `GET /api/novedades` sin token | `200 {"status":"success","version":"0.8.0","novedades":[…]}`; solo texto estático, sin secretos | ✅ |
| `APP_VERSION` (frontend) == `VERSION` (backend) | `0.8.0` en ambos; `version_es_mayor` testeado (`0.8.1 > 0.8.0`); banner/badge lógica revisada | ✅ (UI en navegador pendiente V) |

Estados y DB restaurados (re-seed) y backend detenido al final.

## 4. Disciplina ponytail

| Check | Evidencia | OK |
|---|---|---|
| Cero deps nuevas | `git diff 88398ba main -- '*Cargo.toml'` → vacío | ✅ |
| Sin abstracciones no pedidas | Cambios concentrados en handlers/funciones existentes; 2 archivos nuevos (`novedades.rs` back/front) son los del contrato; `KpiCard`/`CampanitaNovedades` son componentes del brief | ✅ |
| Menor diff que satisface las tareas | `backend/src/main.rs` +8 (mod + ruta); `frontend/.../sidebar.rs` +2; `mod.rs` +1 | ✅ |
| `ponytail:` con techo donde se recorta | `resumen_cartera` (memoria vs agregaciones, con techo). La bump manual de versión queda documentada como techo en `novedades.rs` pero **sin el prefijo literal `ponytail:`** que pedía el plan | ⚠️ menor |
| Clippy limpio | backend y frontend 0/0 | ✅ |

## 5. Seguridad — release gate `skills/security-audit`

Fases: recon del diff (superficie nueva: query params `desde/hasta`, `/api/novedades` pública, reserva atómica `$expr`, recompute de montos), hunt con **2 hunters independientes** (API/inyección/DoS y lógica de negocio), validación adversarial propia de los hallazgos, reporte. **Cero CRITICAL/HIGH.**

| Check | Evidencia en vivo | OK |
|---|---|---|
| Aislamiento de tenant (nuevos endpoints/reserva) | empresa `attacker@evil.test`: abono/pago sobre plan de demo → `404 "Plan no encontrado"`; su dashboard → stats en 0; `?empresa=attacker` no cambia el tenant (sale del JWT) | ✅ |
| Auth de rutas | `dashboard`/`resumen` sin token → `401`; `/api/novedades` es pública por diseño | ✅ |
| Inyección NoSQL en `$expr`/query params | `monto` es `f64` tipado; los campos del `$expr` son referencias `$` fijas; las fechas solo alimentan `parsear_fecha` (nunca Mongo) | ✅ |
| DoS/panic con fechas absurdas | 10 variantes (`0001→9999`, invertidas, `2026-13-45`, `+9999`, duplicadas) → todas `200` en ~2 ms, sin panic; cap de 60 meses | ✅ |
| Validaciones de dinero | `monto` `-5`/`0` → 400; `1e400`/`1e309` → 400 (serde "number out of range"); `plazo` fuera de catálogo → 400 en `evaluar` **y** `autorizar`; cuota duplicada serial → 400 | ✅ |
| XSS | Sin `dangerous_inner_html`; datos del backend en text nodes (`rsx!`); `novedades` es texto estático | ✅ |
| Cero secretos en el diff | `git diff 88398ba main | grep -iE '(password|secret|token|api_key|mongodb+srv|BEGIN …|AKIA|sk-|ghp_)'` → solo comentarios/imports, sin valores | ✅ |
| Licencias | Cero deps nuevas → sin cambio de licencias | ✅ |

### Hallazgos (2 MEDIUM, 1 LOW) — diferidos como excepciones

#### E1 (integridad, MEDIUM, owner: planner) — contador `cobrado` inflado irreversible deja el plan incobrable
La reconciliación de `cargar_cartera` solo sube (`$max`, `credito.rs:231-242`) y el rollback es best-effort (`revertir_reserva`, `credito.rs:825-834`). El guard de escritura usa el contador, no el ledger. Si un `$inc` de reserva queda sin su `Pago` (fallo transitorio de Mongo entre reserva e insert, o crash del proceso) el contador queda **por encima** del ledger para siempre; `$max` nunca lo baja.
**Reproducido (hunter + validado):** plan deuda 3000 con `cobrado:5000`, ledger 0 → `GET /api/creditos` reporta saldo 3000, pero todo `POST /abonos` → `400 "excede el saldo"` y todo `POST /pagos` → `400 "ya está liquidado"`. Sin endpoint de reparación.
**Impacto:** un plan queda incobrable hasta editar Mongo a mano. Confinado al tenant; requiere la ventana de fallo (no es explotable a voluntad). MEDIUM de integridad, no de frontera.
**Fix sugerido (ola 9):** nunca cerrar el camino de cobro por un contador operativo: (a) reintentar/registrar `reconciliacion_pendiente` en el rollback, o (b) reconciliación bidireccional segura (p. ej. contador `reservado`+`confirmado`, o transacción en replica set).

#### E2 (integridad, MEDIUM, owner: planner) — `monto_total` sin cota superior → `pago_mensual: Infinity` persistido corrompe el tablero
`validar_plazo_y_monto` (`credito.rs:303-311`) exige finito y `>0`, pero no acota. `pago_mensual_de` desborda a `inf` con `monto_total` grande pero finito (el `*100` de `redondear2` desborda ~`1e308`).
**Reproducido en vivo (validado por el auditor):** `autorizar {monto_total:1e308, plazo_meses:6, pago_mensual:1, tasa_interes:0}` → `200 OK`; Mongo persiste `pago_mensual: Infinity`; `GET /api/dashboard` → `capital_prestado: null, capital_colocado: null, por_cobrar_neto: null, tasa_morosidad: 6.39e-305` de forma **persistente** y sin recuperación por API.
**Impacto:** una petición autenticada corrompe el dashboard de su propio tenant (no cruza tenants) e incumple el objetivo de E2 ("montos confiables"). MEDIUM.
**Fix sugerido (ola 9):** en `validar_plazo_y_monto`, rechazar `monto_total` por encima de un tope de negocio (p. ej. `1e12`) y/o verificar `pago_mensual_de(...).is_finite()` antes de persistir.

#### O1 (integridad de reporte, LOW, owner: planner) — aging/flujo/por_cobrar ignoran los abonos
`resumen_cartera` decide "cuota pagada" con `pagadas.contains(n)` (ledger de `cuotas`, que en los abonos es `0`), así que aging/flujo/por_cobrar siguen siendo money-overestimados: un abono baja `saldo`/`tasa_morosidad` pero no `aging`. **Reproducido:** plan deuda 3000 fecha 2026-01-01 + abono 500 → `aging.90+ = 3000` (no 2500). Confinado a la misma respuesta; no explotable. Fix sugerido: usar `cuotas_cubiertas` (por dinero) en esas particiones.

### Notas de hardening (NO findings)
- `revertir_reserva` y los `update_one` de `estado` filtran solo por `_id` (sin `empresa`); no explotable porque el `_id` ya salió de la cartera del tenant y un plan ajeno nunca llega a reservar (404).
- `registrar_pago` no valida `monto.is_finite()` (solo el abono lo hace); sin impacto porque `serde_json` rechaza `inf`/`NaN` antes del handler.
- Las rutas protegidas no tienen rate-limit (solo las públicas, por diseño del plan).

### Lo que está BIEN (verificado)
E1 con `find_one_and_update` + `$expr` resiste 5, 10 y 20 abonos concurrentes sin rebasar la deuda (invariante `sum(pagos) ≤ deuda`); el ledger (`pagos`) —no el contador— es lo que alimenta `saldo`/`estado`, así que un contador manipulado no fabrica liquidación. E2 elimina la fabricación de deuda desde el body. Aislamiento de tenant intacto en todos los endpoints nuevos. Fechas de query sin panic. Cero inyección/XSS/secretos.

## Veredicto

**APPROVED WITH EXCEPTIONS** — E1 y E2 son **MEDIUM** de integridad dentro del propio tenant (sin cruce de frontera de seguridad) y O1 es LOW de reporte; documentados con owner y localizados en el planner para la ola 9. El release gate exige cero CRITICAL/HIGH y no hay ninguno: tenant isolation, auth, validaciones, ausencia de inyección/XSS/secretos, la repro de E1 (3×200/2×400), el recompute de E2, el tablero por ventana y `/api/novedades` están verificados en vivo. La ola es desplegable.

**Pendiente pre-ola 9:**
- Cerrar E1 (recuperación del contador `cobrado` inflado) y E2 (tope de `monto_total` / `pago_mensual` finito) y O1 (aging/flujo por dinero).
- Humo UI en navegador (owner V): selector de 5 presets, KPIs, gráficas relabeladas, campanita/banner. Cubierto en lógica por tests + live API, no en navegador.
- Planner: añadir `frontend/src/components/mod.rs` al mapa de propiedad de la ola 8 (wiring obligatorio) y usar el prefijo `ponytail:` en el techo de bump manual de versión.
- V: reparar el toolchain rustup de NixOS (wrapper `ld.lld` roto → builds fallan sin `-fuse-ld=bfd`).
