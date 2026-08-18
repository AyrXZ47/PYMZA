# Auditoría Ola 1 — Cimientos multi-tenant (JWT + frontend en módulos)

Fecha: 2026-08-17 · Auditor: fresh session (independiente del planner)
Árbol: `main` @ `4b5103d` (tras merges `2532cc8` e1 → `aba3ab5` e2, orden del plan)
Fuente de verdad: `.workflow/plan.md` § "Contrato API ola 1" + briefs de los executors.

**Veredicto: APPROVED WITH EXCEPTIONS** (2 excepciones no bloqueantes, owners abajo).

---

## 1. Integridad de integración

| Check | Evidencia | Resultado |
|---|---|---|
| Worktrees mergeados | `git log --oneline`: `2532cc8 merge: wave1-executor-1 backend (JWT real + multi-tenant)` y `aba3ab5 merge: wave1-executor-2 frontend (modulos + contrato ola 1)` en main; parents: e1=`71d6668,28e0510`, e2=`2532cc8,b0b2b84` (orden del plan, sin conflictos: merges de 2 parents) | ✅ |
| `git status` limpio, sin stashes | `git status` → "nothing to commit, working tree clean"; `git stash list` → vacío | ✅ |
| Diff vs plan — todo lo planeado presente, nada fuera del mapa | `git diff --stat 71d6668 2532cc8` → e1 tocó SOLO `backend/Cargo.toml`, `backend/src/**`, `backend/scripts/**`, `docs/API.md`, `.env.example` (su mapa). `git diff --stat 2532cc8 aba3ab5` → e2 tocó SOLO `frontend/src/**` (9 archivos). `frontend/Cargo.toml` NO tocado (prohibido deps; brief cumplido). Nada de la zona "nadie toca" modificado | ✅ |

## 2. Build & tests (árbol integrado)

| Check | Evidencia | Resultado |
|---|---|---|
| Verify brief e1 (`cargo build && cargo test`) | `cargo build` → Finished OK; `cargo test` → `test result: ok. 23 passed; 0 failed` (incluye 4 tests JWT: roundtrip, expirado, malformado/firma corrupta, otro secreto) | ✅ |
| Verify brief e2 (`cargo check --target wasm32-unknown-unknown && cargo test`) | `cargo check --target wasm32-unknown-unknown` → exit 0; `cargo test` → `test result: ok. 8 passed; 0 failed` (bearer header, restauración 401/success/sin-empresa, escaping localStorage, alerta_info) | ✅ |

## 3. Disciplina ponytail (scope)

| Check | Evidencia | Resultado |
|---|---|---|
| Sin deps innecesarias | Única dep nueva: `jsonwebtoken = "9"` en `backend/Cargo.toml` (la pidió el plan/decision log 2026-08-13). `frontend/Cargo.toml` sin cambios (cero deps) | ✅ |
| Sin abstracciones no pedidas | Extractor `EmpresaSession` (FromRequestParts) = exactamente lo del brief; sin middleware genérico, sin refresh tokens, sin roles | ✅ |
| Menor diff; `ponytail:` con techo | `auth.rs` L25-27: HS256/24h con techo (refresh+cookies httpOnly). `api.rs` L22-24: localStorage con techo (cookies httpOnly). `routes/cliente.rs`: reportar guarda correo, techo (enriquecer nombre en lectura). Migración del comentario `ponytail:` viejo de `authed_request` (estaba en el monolito) | ✅ |
| Split cumplido | `frontend/src/main.rs` = 103 líneas (<200): solo `main()`, `App`, `MenuState`, wiring. Módulos: `api.rs` (233), `components/` login 214, alta_cliente 202, plan_modal 255, cartera 81, dashboard 75, sidebar 67, mod.rs 6. Sin router (sigue `MenuState`). Patrón clippy conservado: `let token_val = token();` antes de cada `await` (verificado en las 6 call-sites) | ✅ |
| UI/textos idénticos | Diff de strings literal + conjunto de clases Tailwind viejo vs nuevo: SOLO desaparecieron `/api/creditos/{empresa}`, `/api/dashboard/{empresa}`, `/api/dashboard/x`, `token-temporal-123` (debían morir); clases Tailwind: mismos conjuntos (0 nuevas → no requería regenerar CSS) | ✅ |

## 4. Seguridad

| Check | Evidencia | Resultado |
|---|---|---|
| Trust boundaries validadas | CURP validada (`es_curp_valida`, 7 tests), `motivo` no vacío en reportar ya sin `empresa`. Extractores JWT: token ausente/inválido/expirado/forjado → 401 (probado en vivo abajo) | ✅ |
| Sin secretos commiteados | `rg "JWT_SECRET|eyJ"` en trackeados: `eyJ` = 0 hits (ningún JWT real); único archivo de config con la variable = `.env.example` con placeholder vacío; resto = menciones del nombre en docs/código que la lee. `backend/.env` verificado ignorado (`git check-ignore`). `MONGODB_URI` nunca impreso (usado vía `source .env` + variable) | ✅ |
| Release gate | No aplica: ola 4 es la de release (`skills/security-audit` allí). Check mínimo de ola no-release cumplido | ✅ |
| Licencias | `jsonwebtoken` = MIT; `LICENSE-SOFTWARE` = Apache-2.0 → compatibles | ✅ |

## 5. Audit gate del plan (evidencia curl en vivo, backend corriendo contra Atlas con los demos del humano)

| Check | Evidencia | Resultado |
|---|---|---|
| Login emite JWT real | `POST /api/login` (nueva@empresa.mx) → `status: success`, token 179 chars, ≠ estático ≠ null; claims decodificados: `{sub: nueva@empresa.mx, nombre: Empresa Nueva Demo, exp}`; `exp - now = 86387s` ≈ 24h | ✅ |
| 8 rutas protegidas: 401 sin token, 200 con token | Por ruta (sin token → con token): `GET /api/clientes/:curp` 401→200 · `POST /api/clientes` 401→422* · `POST /api/clientes/:curp/reportar` 401→422*→**success** (body real solo `{"motivo"}`) · `POST /api/creditos/evaluar` 401→422*→**success** (CURP real, tasa 0.03, plan 3 pagos) · `POST /api/creditos/autorizar` 401→422*→**success** (body real sin `empresa`) · `GET /api/creditos` 401→200 · `GET /api/dashboard` 401→200 · `POST /api/ocr` 401→200. *422 = body `{}` inválido: el extractor JWT YA pasó (si fallara sería 401); con body real → 200/success | ✅ |
| `GET /api/creditos` y `/api/dashboard` sin path param | `GET /api/creditos/<email>` y `/api/dashboard/<email>` → **404** (ruta inexistente), con y sin token | ✅ |
| Falsificación rechazada | JWT forjado con `hmac`/secreto `otro-secreto-distinto` (claims legítimos) → `GET /api/dashboard` → **401** | ✅ |
| Aislamiento por tenant | `GET /api/dashboard` con token → `stats.empresa = nueva@empresa.mx` (empresa del token); alerta de reportar guardó `alerta.empresa = nueva@empresa.mx` (correo = tenant key, no nombre); autorizar insertó plan con tenant del token (verificado en código `credito.rs` + flujo en vivo) | ✅ |
| `migrate_tenant.js` idempotente | Corrida 1: "2 empresas procesadas, 6 documentos actualizados" (datos reales aún con `nombre_empresa`); corrida 2: "0 documentos actualizados" → idempotente; tras migrar, el dashboard lee stats reales (3/3600/30) | ✅ |
| `rg "token-temporal-123"` | En código (`backend/`, `frontend/`): **0 hits**. En todo el repo: hits EN DOCUMENTACIÓN — `AGENTS.md` L91/L115 (ver excepción E1) y `.workflow/**` (documenta el contrato viejo y el propio comando del audit gate) | ⚠️ ver E1 |
| `main.rs` <200 líneas + módulos | 103 líneas; `frontend/src/api.rs` + `frontend/src/components/*` (7 módulos) | ✅ |

Nota: los datos de prueba que introdujo el auditor (plan de pago "PRUEBA-AUDITORIA-OLA1", alerta) fueron borrados/revertidos; el dashboard quedó en su estado previo (3/3600/30). La migración de 6 docs era el propósito del script (deuda previa del humo del integrador que usó nombres).

## Excepciones (owners)

- **E1 — `AGENTS.md` quedó desactualizado y ahora es FALSO** (L91 y L115: "token is still static token-temporal-123 (no JWT, routes don't validate it)"). Causa: `AGENTS.md` estaba en la zona "nadie toca" del mapa de propiedad de la ola 1; ningún executor podía actualizarlo. **Owner: planner ola 2** (misma pasada que refresca `docs/ROADMAP.md`, ya anotado en el decision log 2026-08-13). No bloquea.
- **E2 — Humo UI paso 4 pendiente humano** (`cd frontend && dx serve` + navegador: login, persistencia de sesión tras recarga, logout). La lógica está cubierta por unit tests (restauración localStorage, bearer header, logout) y el contrato verificado en vivo por API; la validación en navegador queda como pendiente humano del plan. **Owner: V/humano**. No bloquea la ola 2 (el plan ya la tenía marcada como pendiente).
- Nota (no excepción): el comando `jq .empresa` del plan lee la raíz pero el schema real entrega `stats.empresa` — desajuste del comando, no del comportamiento; ya anotado en el decision log 2026-08-17.

## Conclusión

La ola 1 está completa y correcta sobre el árbol integrado: contrato API cumplido al 100% (JWT real, 8 rutas protegidas con 401, tenant key = correo, rutas/bodies sin `empresa`), split frontend limpio sin pérdida de UI ni deps, tests en verde en ambos crates, y humo e2e en vivo verificado por el auditor. Las 2 excepciones tienen owner y no bloquean la planificación de la ola 2.