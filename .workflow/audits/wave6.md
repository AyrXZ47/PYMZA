# Auditoría Ola 6 — Contrato PDF + Producción (RELEASE GATE)

- **Fecha:** 2026-09-06 (sesión de auditoría en fresco, árbol integrado `main` @ `99fd4b5`)
- **Alcance:** `.workflow/plan.md` secciones "Ola 6" y "Audit gate (RELEASE GATE)", briefs `wave6-executor-{1,2}.md`, `.workflow/audit-checklist.md`, + `skills/security-audit` (6 fases) sobre el árbol integrado
- **Veredicto: REJECTED** — 2 HIGH confirmados en vivo (F1/F2: DoS por `plazo_meses` sin validar) + `Dockerfile.backend` no construye (2 bugs, el humo Docker pendiente del plan). Fix de ~15 líneas + re-auditoría puntual. El tenant isolation del PDF PASA y todo lo demás del gate está en verde o con excepciones LOW/MEDIUM con owner.
- **Artefactos:** `wave6-security-report.md` (reporte del security-audit), `wave6-security-findings.json` (structured output, validado con `validate-findings.cjs` — PASS 10/10)

---

## 1. Integridad de la integración

| Check | Evidencia | OK |
|---|---|---|
| Worktrees mergeados a main | `git log --oneline main..wave6-executor-1` y `main..wave6-executor-2` → **0 commits sin mergear** en ambas | ✅ |
| `git status` limpio, sin stashes | `git status --porcelain` → sin salida; `git stash list` → 0 | ✅ |
| Diff vs plan: nada fuera del mapa | `git diff ca79370..main --stat` → 18 archivos, 100% dentro del mapa: EJ-1 (`backend/Cargo.toml`, `backend/src/{auth,main,ocr,pdf,routes/credito}.rs`, `backend/scripts/fixture_recibo.png`, `docs/API.md`, `.env.example`, `Dockerfile.backend`), EJ-2 (`frontend/src/{api.rs,components/{cartera,plan_modal}.rs}`, `Dockerfile.frontend`, `docker-compose.yml` (solo args de frontend, verificado en diff), `docs/DEPLOY.md`, `README.md` (1 línea, solo el enlace)), + `.workflow/plan.md` (integrador, estado — permitido) | ✅ |
| Todo lo planeado presente | pdf.rs (186 l), endpoint contrato, CORS env, body limit 3MB, rate limit, fixture_recibo.png (39 KB), Dockerfiles, DEPLOY.md (140 l), botón descarga (cartera + plan_modal) | ✅ |

Commits de la ola: EJ-1 `ec3484b, da611e4, b0c31b2, bb96df9`; EJ-2 `7888e2f, d6f979a, 4e7e1cf, bd3553a`; merges `aaae8e4, 5333eca`; docs integrador `99fd4b5`.

## 2. Build y tests (árbol integrado)

| Comando | Salida | OK |
|---|---|---|
| `cd backend && cargo build && cargo test` | `Finished dev profile` → `test result: ok. 68 passed; 0 failed` (60 de la ola 5 + 8 nuevos: pdf, parseo CORS, límites rate) | ✅ |
| `cd frontend && cargo check --target wasm32-unknown-unknown && cargo test` | `Finished dev profile` (wasm) → `test result: ok. 40 passed; 0 failed` (36 previos + 4 nuevos: Content-Disposition, descarga) | ✅ |
| Verify EJ-2 tailwind | ver hallazgo S2 abajo — el CSS compilado NO se regeneró | ⚠️ |

## 3. Audit gate (evidencia en vivo)

Entorno: mongod local (`--dbpath /tmp/opencode/mongo-data`, puerto 27017), `seed.js` fresco, backend compilado de `main` con `MONGODB_URI` local + `JWT_SECRET`/envs efímeras (`dotenvy` no sobreescribe envs existentes — verificado en `main.rs:89`). **Nota ambiental:** `mongo:latest` (8.x) NO arranca en este kernel (`MongoDB cannot start: Linux kernel versions 6.19 and newer has a known incompatibility... SERVER-121912`) — el compose `mongo` no sirve en local hasta fijar versión; el AGENTS.md ya documenta el fallback (`mongod` local) que se usó aquí y en la ola 5. Entorno limpiado al final (mongod shutdown + dbpath borrado).

| Check del plan | Resultado | OK |
|---|---|---|
| **B. Contrato PDF 200** | 200, `content-type: application/pdf`, `content-disposition: attachment; filename="contrato-RAMJ920215MDFMZR05.pdf"`, `head -c 8` → `%PDF-1.3`, 4369 B | ✅ |
| **B. Datos del tenant en el PDF** | Texto extraído del stream (hex WinAnsi→latin1): título "CONTRATO DE CRÉDITO" (con acento), fecha emisión 2026-09-06, "Nombre: Ferretería El Tornillo", "Correo: demo@pymza.mx", "Nombre: Janeth Ramos Zamora", "CURP: RAMJ920215MDFMZR05", producto/monto/plazo/tasa/pago mensual, **tabla completa de 6 filas (mes, pago, interés, capital, saldo)**, línea de firma, leyenda "Contrato generado por PYMZA" | ✅ |
| **B. 401 sin token** | `→ 401` | ✅ |
| **B. 404 plan ajeno/inexistente** | `{"status":"error","message":"Plan no encontrado"} → 404` (hex 000…0); `no-es-hex` → `400 "plan_id inválido: se espera el hex del ObjectId"` | ✅ |
| **B. Tenant isolation (foco del gate)** | Empresa B registrada+logueada: contrato del plan de demo → **404**; cartera de B → vacía. Cross-tenant: IMPOSIBLE | ✅ |
| **C. CORS default dev** | `Origin: https://evil.com` → sin `access-control-allow-origin`; `http://localhost:8080` → `allow-origin: http://localhost:8080` | ✅ |
| **C. CORS con env** | `ALLOWED_ORIGINS="https://app.pymza.mx, http://localhost:8080"` (con espacio tras coma): evil.com → 0 allow-origin; app.pymza.mx y localhost:8080 → allow-origin presente (trim OK); **sufijo host** `https://app.pymza.mx.evil.com` → 0 (sin bypass) | ✅ |
| **D. E1 cerrada (>2MB → 400)** | PNG real de 2,236,488 B (b64 2,981,984 < 3MB global) → POST recibos → **`{"status":"error","message":"El archivo excede el máximo de 2 MB"}` → 400** (no 413). El 413 queda como red de seguridad >3MB | ✅ |
| **E. Rate limit 429** | Defaults (10 rps/burst 20): ráfaga de 25 → `200×20, 429×5` (coincide con docs/API.md; el "11º" del script de V asumía 10/60s — el 429 llega a la 21ª); con `RATE_LIMIT_RPS=1 RATE_LIMIT_BURST=5`: `200×5, 429×6` — **de 11 logins, el 11º es 429**. Body: `{"status":"error","message":"Demasiadas peticiones, intenta más tarde"}` + header `x-ratelimit-after: 1`. **Envs gobiernan** (probado con valores no-default). Protegidas sin límite: 25 GET /api/dashboard → 200×25 | ✅ |
| **F. fixture_recibo.png (E2 cerrada)** | `tesseract fixture_recibo.png -` → "RECIBO DE SERVICIO / SERVICIO: LUZ … **TOTAL: $450.00 MXN**"; test `buscar_monto_lee_el_output_real_del_fixture_recibo` ✅ (5 tests de buscar_monto OK); **humo end-to-end**: POST recibos con fixture → `monto_leido: 450.0, score 720→745 (Medio)`; 2º (agua) → `770 → Bajo`; 3º → `400 "Máximo 2 recibos por cliente"`, `recibos.count()=2` | ✅ |
| **G. docker compose build** | Frontend: **OK** (`pymza-frontend` construida). Backend: **FALLA** — ver hallazgo S1 | ❌ |
| **G. tesseract en imagen backend** | Con Dockerfile de diagnóstico (fixes de S1): `tesseract 5.3.0`, langs `eng, osd, spa` — la receta de runtime es correcta una vez resuelto S1 | ✅ (tras fix) |
| **G. API_BASE inyectado** | `docker build --build-arg API_BASE=https://audit-api.pymza.mx` → `grep -ao 'https://audit-api.pymza.mx' assets/frontend_bg-*.wasm` → **match** (inyectado en el WASM) | ✅ |

## 4. Disciplina ponytail

| Check | Evidencia | OK |
|---|---|---|
| Cero deps nuevas salvo printpdf y tower_governor | `git diff ca79370..main -- backend/Cargo.toml frontend/Cargo.toml` → solo `printpdf = "0.7.0"` y `tower_governor = "0.1.0"` (frontend sin cambios). Versiones justificadas con comentario (printpdf 0.7 por compatibilidad; tower_governor 0.1 = última compatible con axum 0.6) | ✅ |
| Sin abstracciones no pedidas | pdf.rs función pura sin motor de layout; rate limit solo en 2 rutas públicas; PDF bajo demanda (no se almacena); backups = feature de Atlas (0 código) | ✅ |
| Diff mínimo | Backend ~370 l (de ellas ~120 tests), frontend ~250 l, DEPLOY.md 140 l | ✅ |
| `ponytail:` con techo | `pdf.rs:4-5` (regeneración bajo demanda; techo firma/logo), `pdf.rs:34-36` (tabla ≤12 meses; techo paginación), `auth.rs:27-28` (techo refresh tokens), `auth.rs:182-184` (argon2 sync; techo spawn_blocking), `main.rs:32-33` (techo extender a OTP), `Dockerfile.frontend:27-30` (cache API_BASE; techo archivo generado), `api.rs` (techo localStorage documentado) | ✅ |
| Licencias deps nuevas | printpdf 0.7.0 = **MIT**; tower_governor 0.1.0 = **MIT OR Apache-2.0** (Cargo.toml del registry) — compatibles con LICENSE-SOFTWARE | ✅ |

## 5. Seguridad mínima (checklist base)

| Check | Evidencia | OK |
|---|---|---|
| Trust boundaries validadas | kyc/recibos: mime → tamaño por largo b64 → base64 → 404 (E1 probada en vivo); plan_id hex→400; Content-Disposition con fallback si el curp trae bytes no visibles (credito.rs:728-732) | ✅ |
| Sin secretos commiteados | Scan `mongodb+srv|sk-|EAAG` en DEPLOY.md/.env.example/README.md → solo placeholders documentales (`usuario:password@`, `user:pass@`, "cambia-este-secreto"); `.env.example` con `ALLOWED_ORIGINS/RATE_LIMIT_*` vacías + comentarios de defaults | ✅ |
| Release gate `skills/security-audit` | Corrido completo (6 fases) — ver §6 | ❌ HIGH → REJECTED |

## 6. RELEASE GATE — security-audit (6 fases)

- **Fase 1 (recon):** mapa de 17 rutas, trust boundaries (JWT público/protegido, path params, body b64, envs), baseline (API de crédito multi-tenant; comparable: API de préstamos PYME).
- **Fase 2 (hunt):** 2 agentes paralelos (backend: NoSQL/panics/dinero/OTP/infra; frontend: URLs/sesión/XSS/download) + hunt propio dinámico (tenant PDF, XFF, CORS, b64).
- **Fase 3 (validación adversarial):** pruebas EN VIVO contra el backend local (lista de cuerpos y outputs en `wave6-security-report.md`).
- **Fase 4/5:** `wave6-security-report.md` + `wave6-security-findings.json` **validado con validate-findings.cjs → PASS 10/10**.
- **Fase 6 (verificación independiente):** 6 agentes frescos (uno por finding F1–F6) — F1/F2/F4/F8 CORRECTED (matiz de precisión aplicado), F3/F5/F6/F7 CORRECT; refutados por evidencia: `1e400`→400 del extractor y emoji→PDF limpio.

| ID | Hallazgo | Severidad | Evidencia |
|---|---|---|---|
| F1 | **DoS total con 1 request**: `evaluar` con `plazo_meses` gigante → `collect()` de ~86 GB → OOM del proceso (RSS 20→30 GB en 12 s, medido; proceso terminado a mano) | **HIGH** | en vivo |
| F2 | **Plan envenenado persistido**: `autorizar` inserta `plazo_meses` sin validar → cartera/resumen congelados (GET cartera timeout 6 s, 66 % CPU), dashboard stale permanente, contrato-PDF = OOM persistente; sin endpoint de borrado | **HIGH** | en vivo |
| F3 | `telefono_verificado` falsificable: OTP nunca compara el teléfono con el del cliente (cliente con tel. real 5551234567 verificado vía OTP a 5550000001); cross-tenant | MEDIUM | en vivo |
| F4 | Rate limiter por peer-addr: en Railway el bucket es GLOBAL (la doc del crate lo advierte); sin bypass por XFF (probado), pero un atacante puede 429-ear el login de todos. Requires deployment testing | MEDIUM | código + crate docs |
| F5 | Carrera `alta_empresa` sin índice único: dos registros simultáneos → tenant key compartida → cross-tenant total para ese correo | MEDIUM | código (race ~100 ms argon2) |
| F6 | TOCTOU `registrar_pago`: cuota duplicada, cobrado inflado, "Liquidado" falso (`.len()` cuenta duplicados) | MEDIUM | código |
| F7 | Enumeración de correos (respuesta "Ya existe" + timing argon2 ~100 ms en login) | LOW | código |
| F8 | Carrera tope de recibos (N2 ola 5 persiste); precisión: el score SÍ decide `evaluar` (banda $2000-5000), aunque `autorizar` no lo consulta | LOW | código |

**Lo que pasa:** tenant isolation del PDF (query `_id`+`empresa`, probado con empresa B → 404), inyección NoSQL limpia (35 sinks), JWT/argon2 sólidos, CORS sin bypass, XSS cero (0 raw HTML), descarga de contrato segura (filename DOM-saneado), 1e400/emoji rechazados limpio. Detalle completo en `wave6-security-report.md`.

## 7. Hallazgos de la ola (fuera del security-audit)

### S1 — `Dockerfile.backend` NO construye (bloqueante del despliegue; owner: executor-1/ola 6-fix)
El humo Docker quedó pendiente en la sesión del executor (socket sin grupo; plan lo anota) y la auditoría lo ejecutó:
1. `COPY backend/Cargo.toml backend/Cargo.toml` copia a `/build/backend/Cargo.toml`, no a `/build/` → `error: could not find Cargo.toml in /build` (evidencia del build).
2. `FROM rust:1.83-bookworm` ya no compila las deps actuales: `failed to parse manifest ... time-core-0.1.9` (edition2024 — mismo problema que EJ-2 resolvió en `Dockerfile.frontend` subiendo a `rust:1.97`, commit bd3553a).
**Fix validado por la auditoría** (Dockerfile de diagnóstico en /tmp, NO commiteado): `COPY backend/Cargo.toml ./Cargo.toml` + `COPY backend/src ./src` + `FROM rust:1.97-bookworm AS builder` → build OK, imagen con `tesseract 5.3.0 + spa`, usuario no-root `app`. **Railway desplegará con este Dockerfile: sin el fix no hay despliegue.**
También hallazgo ambiental para V: `mongo:latest` (8.x) del compose no arranca en kernel ≥6.19 (SERVER-121912) — en Railway/Atlas no aplica; en local, fijar versión o usar el `mongod` documentado.

### S2 — CSS compilado no regenerado (menor; owner: executor-2/ola 6-fix)
El botón "Descargar contrato" usa 4 clases que faltan en `frontend/assets/tailwind.css` (último commit del CSS es de la ola 5, `0b68b24`): `hover:bg-blue-700`, `py-1.5`, `hover:bg-slate-300`, `dark:bg-slate-700` (verificado con grep, 0 matches). El botón funciona (bg base sí está) pero se ve incompleto en hover/dark. El brief lo pedía explícitamente ("Si el botón añade clases Tailwind nuevas → regenera CSS y commitéalo"). Fix: `cd frontend && ./tailwind.sh` + commit del CSS.

## 8. Veredicto

**REJECTED** — regla del release gate: cero CRITICAL/HIGH sin excepción, y APPROVED WITH EXCEPTIONS solo admite excepciones LOW/MEDIUM con owner. F1/F2 son HIGH confirmados en vivo (DoS total con 1 request autenticado / estado persistido que congela el servicio) y el `Dockerfile.backend` no construye (el despliegue a Railway fallaría de frente). Nada de esto es grande de arreglar:

**Para pasar la ola (en orden):**
1. **Fix F1+F2** (executor-1): validar `plazo_meses ∈ 3..=12` y `monto > 0` (finito) en `evaluar_credito` y `autorizar_credito` → 400 con mensaje del contrato. ~15 líneas + tests.
2. **Fix S1** (executor-1): las 3 líneas del `Dockerfile.backend` (`COPY ./Cargo.toml`, `COPY ./src`, `rust:1.97`) — ya validadas por esta auditoría.
3. **Fix S2** (executor-2): `./tailwind.sh` + commit del CSS compilado.
4. **Re-auditoría puntual** (auditor): `cargo test`, los 3 payloads de F1/F2 → 400, `docker compose build` OK + tesseract en imagen. No hace falta repetir el resto del gate.
5. F3–F8 + hardening: quedan en el ledger (olas 7+); F3/F4/F5/F6 con fixes baratos si V quiere subir el listón pre-despliegue.

---

## Re-auditoría 2026-09-06 — ola 6-fix

- **Fecha:** 2026-09-06 (sesión de auditoría en fresco; árbol integrado `main` @ `12ea9b2`, `git status` limpio)
- **Alcance puntual:** solo cierra F1/F2 (HIGH), S1, S2 y T4/F5 (aprobado por V). NO repite el gate completo de la ola 6.
- **Commits auditados (diff `4c6f0e9..main`):** `a32b096` (F1/F2: `credito.rs`), `ab551d0` (S1: `Dockerfile.backend`), `31487bb` (T4/F5: `db.rs`), `6101aa1`/`c5a2877`/`12ea9b2` (docs plan/brief).
- **Veredicto: APPROVED WITH EXCEPTIONS** — F1/F2/S1/S2/T4 cerrados y verificados en vivo; una excepción LOW documentada abajo (preexistente, fuera del scope del fix).

### Checks (comando → salida)

1. **Backend build+test:** `cd backend && cargo build && cargo test` → `Finished dev profile` → `test result: ok. 73 passed; 0 failed; 0 ignored` (68 previos + 5 nuevos de F1/F2: tests inline `evaluar_rechaza_plazo_gigante_con_400`, `evaluar_rechaza_monto_negativo_con_400`, `autorizar_rechaza_plazo_gigante_con_400`, `autorizar_rechaza_monto_negativo_con_400`, `validar_plazo_y_monto` bordes 3..=12, log en `/tmp/opencode/backend-test.log`).
2. **Frontend check+test (wasm):** `cd frontend && cargo check --target wasm32-unknown-unknown && cargo test` → `Finished` (wasm) → `test result: ok. 40 passed; 0 failed`.
3. **Payloads del contrato en vivo** (mongod local `--dbpath /tmp/opencode/mongo-data`, seed fresco, `cargo build` de `main`, backend en `BIND_ADDR=127.0.0.1:3100` porque el :3000 del host estaba ocupado por un proceso preexistente):
   - `POST /api/login` demo@pymza.mx/demo1234 → token (181 chars). ✅
   - **F1.1** `evaluar` `{"monto":10000,"plazo_meses":1000000,...}` → `{"status":"error","message":"El plazo debe estar entre 3 y 12 meses"}` **HTTP 400** (proceso vivo; sin OOM). ✅
   - **F1.2** `evaluar` `{"monto":-1,"plazo_meses":6,...}` → `{"status":"error","message":"El monto debe ser mayor a 0"}` **HTTP 400**. ✅
   - **F2** `autorizar` plan envenenado `plazo_meses:1000000` → **HTTP 400** "El plazo debe estar entre 3 y 12 meses"; `db.planes_pago.countDocuments()` **antes=1, después=1** (el plan del seed; NADA se insertó). ✅
   - **T4/F5.1** `db.empresas.getIndexes()` → `{ key: { correo: 1 }, name: 'correo_1', unique: true }` — índice único creado por el backend al arrancar. ✅
   - **T4/F5.2** POST `/api/empresas` correo duplicado (demo y también una creada durante la prueba) → `{"status":"error","message":"Ya existe una empresa registrada con ese correo"}`, mensaje claro; `db.empresas` sin duplicados (2 docs: demo + 1 nueva creada en la prueba). ✅
4. **Docker (S1):** `docker compose build` → `Image pymza-backend Built`, `Image pymza-frontend Built` (ambos, sin errores). El log muestra el fix activo: `[backend builder] FROM ... rust:1.97-bookworm`, `COPY backend/Cargo.toml ./Cargo.toml`, `COPY backend/src ./src` (las 3 recetas de la auditoría ola 6). `docker run --rm pymza-backend tesseract --version` → `tesseract 5.3.0`; `spa.traineddata` presente; imagen corre como usuario no-root `app` (uid 10001). ✅
5. **CSS (S2 — falso positivo del grep, confirmado):** `grep -c "hover:bg-blue-700\|..." frontend/assets/tailwind.css` → **0** con patrón literal. Las 4 clases SÍ están en el CSS: Tailwind v4 escapa los selectores; con el patrón correcto (escapado) matches 1 por clase: `hover\:bg-blue-700 ×1`, `py-1\.5 ×1`, `hover\:bg-slate-300 ×1`, `dark\:bg-slate-700 ×1`. El código fuente sigue usándolas (cartera.rs:178,191 — exactamente esas 4). **S2 cerrado sin commit de CSS** (regeneración byte-idéntica, hash `15cdb136` según T3 del plan — consistente con mi grep: nada cambió). ✅
6. **Diff del fix vs mapa:** `git diff 4c6f0e9..main --stat` → 5 archivos: `backend/src/routes/credito.rs` (+123/−12), `Dockerfile.backend` (+8/−3), `backend/src/db.rs` (+23), `.workflow/plan.md` y `.workflow/briefs/wave6fix-executor-1.md` (docs — permitidos). 100% dentro del mapa de la 6-fix. `backend/Cargo.toml`/`frontend/Cargo.toml` sin cambios → **cero deps nuevas**. Los fixes llevan `ponytail:` comments (Dockerfile: techo del tag rust; db.rs: falla blanda idempotente con reintento al arranque). ✅

### Excepciones (LOW, no bloquea la ola)

| ID | Hallazgo | Severidad | Owner |
|---|---|---|---|
| A6-1 | `POST /api/empresas` con correo duplicado responde **HTTP 200** con `{"status":"error",...}` (no 4xx). Es el contrato documentado Y preexistente del endpoint (docs/API.md §`/api/empresas` — errores por body); no es introducido por el fix. El objetivo de T4/F5 queda cerrado: índice único en DB + mensaje claro + sin_dupes → la carrera ya no comparte tenant key. Códigos HTTP coherentes (409/422) → olas 7+ junto con F7. | LOW | V (ola 7) |

### Cierre release gate

F1/F2 (los únicos HIGH del security-audit) quedan CORREGIDOS y verificados en vivo. S1 (bloqueante de despliegue) construye ambas imágenes y la imagen backend tiene tesseract 5.3.0+spa+usuario no-root. S2 era falso positivo (grep sin escapar selectores de Tailwind v4). T4/F5(aprobado por V) cerrado con índice único. El resto del security-audit sigue en verde (ola 6, §3) y lo pendiente (F3, F4, F6, F7, F8) queda en el ledger para olas 7+. **V puede desplegar siguiendo `docs/DEPLOY.md`.**
