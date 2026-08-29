# Auditoría Ola 2 — Portal público (landing + registro/login + tema + API_BASE)

Fecha: 2026-08-28 · Auditor: fresh session (independiente del planner)
Árbol: `main` @ `406f80c` (tras merges `5259426` e1 → `125e59d` e2, orden del plan)
Fuente de verdad: `.workflow/plan.md` § "Ola 2 (actual)" + briefs wave2-executor-1/2.

**Veredicto: APPROVED WITH EXCEPTIONS** (2 excepciones no bloqueantes, owners abajo).

---

## 1. Integridad de integración

| Check | Evidencia | Resultado |
|---|---|---|
| Worktrees mergeados en orden del plan | `git log --graph`: `5259426 merge: wave 2 wave2-executor-1 (frontend portal publico)` → `125e59d merge: wave 2 wave2-executor-2 (docs post-ola 1)`; base común `964118e` (plan ola 2). Orden e1→e2 tal como el plan de integración manda | ✅ |
| `git status` limpio | `git status` → "nothing to commit, working tree clean"; branch `main` up to date con `origin/main` | ✅ |
| Stashes | `git stash list` → 1 stash: `idea diferida not-paid (rescatable)` — contenido: SOLO 4 líneas en `docs/ROADMAP.md` (nota de idea diferida 2026-08-27, marcada "no tiene tarea en plan"). No es trabajo de la ola retenido. Ver excepción E1-stash | ⚠️ ver E1 |
| Diff vs plan — executor-1 solo su mapa | `git diff --stat 964118e 5259426` → 14 archivos, TODOS de `frontend/src/**` (api, main, components incl. nuevos landing/registro/theme_toggle), `frontend/tailwind.css`, `frontend/assets/tailwind.css`. NO tocó `frontend/Cargo.toml`, `tailwind.sh`, `Dioxus.toml`, `frontend/AGENTS.md` (prohibidos) | ✅ |
| Diff vs plan — executor-2 solo su mapa | `git diff --stat 5259426 125e59d` → SOLO `AGENTS.md`, `README.md`, `docs/API.md`, `docs/ROADMAP.md` (su mapa exacto) | ✅ |
| Nada fuera del mapa combinado | `git diff --name-only 964118e main` filtrado → 0 archivos fuera de `frontend/`, `AGENTS.md`, `README.md`, `docs/`; `git diff --stat 964118e main -- backend/` → **vacío** (backend cero cambios, como el plan exige). `.workflow/` solo tocado por planner (`964118e`) e integrador (`406f80c`, 1 línea del estado de la tabla) | ✅ |
| Branch isolation | Commits por rama: e1 = `df412e2,2c888a2,70b9750,9b0c1f4`, e2 = `399d3a2`; merges de 2 parents, sin commits de executors sobre main ni ramas ajenas | ✅ |

## 2. Build & tests (árbol integrado, corridos por el auditor)

| Check | Evidencia | Resultado |
|---|---|---|
| `cd backend && cargo build && cargo test` | `Finished dev profile` OK; `test result: ok. 23 passed; 0 failed` — misma cifra que ola 1 (backend inalterado ✓) | ✅ |
| `cd frontend && cargo check --target wasm32-unknown-unknown` | `Finished dev profile` OK (exit 0) | ✅ |
| `cd frontend && cargo test` | `test result: ok. 11 passed; 0 failed` — los 8 de ola 1 intactos (`authed_request`, `evaluar_restauracion`×3, `alerta_info`×3, `js_storage`) + 3 nuevos de tema: `theme_invertir_alterna_light_y_dark`, `theme_invertir_desconocido_cae_a_light`, `theme_class_js_activa_dark_solo_con_dark` (lo que el brief pedía: lógica de tema pura testeable en host) | ✅ |
| `./tailwind.sh` — CSS commiteado en sync | Regeneración OK (tailwindcss v4.3.3) y `git status`/`git diff` **vacíos tras regenerar**: el `assets/tailwind.css` commiteado es byte-idéntico a la regeneración (check más fuerte que el `git diff --stat` del brief) | ✅ |
| Verify brief e2 (docs) | `! rg "token-temporal-123\|/api/creditos/:empresa\|/api/dashboard/:empresa" AGENTS.md README.md docs/ && rg -q "JWT\|jwt" ...` → **"docs OK"** | ✅ |

## 3. Audit gate del plan (evidencia de comandos)

| Check | Evidencia | Resultado |
|---|---|---|
| Sin auth: landing, no login | `main.rs`: `vista_publica = use_signal(\|\| VistaPublica::Landing)` (L48); `if is_authenticated() {app} else {match vista_publica() {Landing→…}}` (L97–133). El default sin sesión es Landing | ✅ (código; humo navegador = E2) |
| Registro → auto-login | `registro.rs`: `POST /api/empresas` → si `status==success` → `POST /api/login` con mismas credenciales → `token_guardar` + `current_company` + `is_authenticated=true`; si auto-login falla → aviso ámbar + botón "Ir a Iniciar sesión" (L129–141). Verificado en vivo: `POST /api/empresas` duplicado → `{"status":"error","message":"Ya existe una empresa registrada con ese correo"}`; `POST /api/login` (empresa demo del plan) → `status: success`, `empresa: Empresa Nueva Demo`, token 179 chars, JWT (3 segmentos) — exactamente el shape que `registro.rs` lee (`data_login["empresa"]`, `data_login["token"]`) | ✅ |
| Enlaces cruzados + login sin form embebido | `login.rs`: 0 restos de `reg_*` ni "Registrar Empresa" (grep); solo enlaces a `VistaPublica::Registro` y `VistaPublica::Landing`. `landing.rs`: CTAs "Crear cuenta"→Registro e "Iniciar sesión"→Login (hero + beneficios presentes, 102 líneas). `registro.rs`: "¿Ya tienes cuenta? Inicia sesión"→Login | ✅ |
| `rg "token-temporal-123"` → 0 | `grep -rn token-temporal-123 frontend/src backend/src AGENTS.md README.md docs/` → **0 hits**. **E1 de la ola 1 saldada** (`AGENTS.md` ya describe JWT real, 8 rutas protegidas, tenant=correo) | ✅ |
| Rutas viejas `:empresa` muertas → 0 | Verify exacto del brief (`/api/creditos/:empresa\|/api/dashboard/:empresa`) → 0 ("docs OK"). El patrón suelto `:empresa` del plan da 3 hits PERO todos son identificadores Rust (`use crate::models::empresa::Empresa`, `routes::empresa::`) — falsos positivos, no rutas. Nota: patrón del audit gate impreciso; el exacto es el correcto | ✅ (ver nota) |
| Tema: `dark:` migrado + variante v4 | `@custom-variant dark (&:where(.dark, .dark *));` en `frontend/tailwind.css` L2 (Tailwind v4). `grep -rc "dark:" frontend/src --include="*.rs"` → presentes en los 10 archivos de UI (alta_cliente 21, cartera 6, dashboard 12, landing 12, login 9, plan_modal 25, registro 12, sidebar 7, theme_toggle 1, main 7). CSS regenerado y commiteado (§2) | ✅ |
| Persistencia tema + toggle | `api.rs`: `THEME_STORAGE_KEY="pymza_theme"`; `theme_invertir` (puro, testado ×2), `theme_aplicar` (classList.toggle en `<html>` vía `document::eval`), `theme_leer/guardar` cfg-gated wasm. `main.rs`: default `"dark"` (look actual, decision log), restauración + aplicación en `use_effect`. Toggle compartido `theme_toggle.rs` (25 líneas) usado por sidebar y landing | ✅ (código; persistencia tras recarga = E2 navegador) |
| `API_BASE` configurable | `api.rs` L13–16: `pub const API_BASE: &str = match option_env!("API_BASE") { Some(url) => url, None => "http://127.0.0.1:3000" }` — equivalente const al `unwrap_or` del plan; doc-comment con el plan de Railway (ola 4) | ✅ |
| Boundary de auth en vivo | `GET /api/dashboard` sin token → **401**; login con credenciales malas → `Credenciales inválidas`. Backend corre contra Atlas en este árbol | ✅ |

## 4. Disciplina ponytail (scope)

| Check | Evidencia | Resultado |
|---|---|---|
| Cero deps nuevas | `git diff 964118e main -- frontend/Cargo.toml backend/Cargo.toml` → **vacío**. Tema resuelto con clase `dark` + localStorage (lo mínimo; sin framework de estados, sin router — decision log 2026-08-17) | ✅ |
| Sin abstracciones no pedidas | Componentes nuevos = exactamente los del brief (`landing`, `registro`, `theme_toggle`). `theme_toggle` es un componente compartido real (sidebar + landing), no abstracción speculative. Sin helpers de tema genéricos: 5 funciones planas en `api.rs` con lógica pura separada para testear | ✅ |
| Menor diff | +969/−461 en 14 archivos: 258 líneas son los 2 archivos nuevos pedidos, ~350 son pares `dark:` (migración pedida archivo por archivo), 620 del CSS regenerado. `login.rs` bajó de 214 a 118 (el form de registro se MOVIÓ, no se duplicó). `main.rs` 135 líneas (<200, mantiene el split de ola 1) | ✅ |
| `ponytail:` con techo | `main.rs` L3: sin router, techo "router cuando existan URLs públicas reales tras desplegar el portal, ola 4". `api.rs` L29: localStorage token, techo "sin refresh tokens ni cookies httpOnly". Ambos nombran upgrade path | ✅ |
| Paridad de validación | El form de registro movido no tenía validación client-side de password ≥8 en el original (solo placeholder) — el movimiento no perdió nada; el backend valida (`routes/empresa.rs` rechaza inválidos, verificado en vivo con duplicado) | ✅ |

## 5. Seguridad

| Check | Evidencia | Resultado |
|---|---|---|
| Sin secretos commiteados | `git diff 964118e main` escaneado (`JWT_SECRET=`, URIs mongo con credencial, `eyJ…`): únicos hits = placeholders de ejemplo (`mongodb+srv://user:pass@…`, `JWT_SECRET="cambia-este-secreto"` en README — instrucciones de setup, preexistentes en forma `echo`, executor-2 solo cambió a `printf` y añadió la línea del placeholder). Sin JWTs reales. `git check-ignore backend/.env` → ignorado; únicos `.env` trackeados = `.env.example` con valores vacíos | ✅ |
| Trust boundaries | `/api/empresas` (pública) valida correo/password/nombre y duplicados (verificado en vivo); `/api/login` rechaza credenciales malas (verificado en vivo); rutas protegidas → 401 sin token (verificado en vivo). Frontend: el registro consume los endpoints públicos existentes, sin trust boundary nueva | ✅ |
| Release gate | No aplica: ola 4 es la de release (`skills/security-audit` allí), igual que ola 1 | ✅ |
| Licencias | Cero deps nuevas → nada que revisar | ✅ |

## Excepciones (owners)

- **E1 (stash) — `stash@{0}: idea diferida not-paid (rescatable)`**: el checklist pide cero stashes. Contenido inspeccionado: solo 4 líneas de docs en `ROADMAP.md` (idea "not-paid" del 2026-08-27, marcada "no tiene tarea en plan", con su `ponytail:` y techo). No es trabajo de la ola ni código. **Owner: V** — aplicar como `docs:` (o descartar) antes de que arranque la ola 3 para dejar el árbol sin stashes. No bloquea.
- **E2 (heredada de ola 1) — Humo UI en navegador pendiente humano** (`dx serve`: landing sin login, registro con auto-login real, enlaces cruzados, toggle de tema con persistencia tras recarga, flujo completo). Cubierto por: código verificado línea a línea, tests de la lógica pura, CSS en sync verificado por regeneración, y las 2 patas del auto-login probadas por API en vivo. Pero la validación visual en navegador es la única evidencia que salda E2 de la ola 1 (el plan: "Si el humo UI pasa, la excepción E2 de la ola 1 queda saldada"). **Owner: V/humano**. No bloquea la ola 3 (ya estaba marcada como pendiente humano en la tabla de olas).

Nota (no excepción): el comando del audit gate `rg ":empresa"` produce falsos positivos con identificadores Rust (`models::empresa`); el patrón exacto `/api/creditos/:empresa|/api/dashboard/:empresa` es el correcto — sugerencia para el planner al redactar los gates de las próximas olas.

## Conclusión

La ola 2 está completa y correcta sobre el árbol integrado: landing por defecto sin auth, registro con auto-login (shape de login verificado en vivo), login purgado del form embebido, tema claro/oscuro migrado con variante Tailwind v4 y CSS regenerado byte-idéntico al commiteado, `API_BASE` configurable por build, docs al estado real (E1 saldada), backend intacto (cero cambios, tal como el plan exige), cero deps nuevas, tests 23+11 en verde. Las 2 excepciones tienen owner y no bloquean la planificación de la ola 3.
