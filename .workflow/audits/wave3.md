# Auditoría Ola 3 — Identidad verificable (CURP robusta + correo del cliente + OTP por teléfono)

Fecha: 2026-08-31 · Auditor: fresh session (independiente del planner)
Árbol: `main` @ `2739473` (tras merges `446ded2` e1 → `a77a805` e2, orden del plan; integración `2739473`)
Fuente de verdad: `.workflow/plan.md` § "Ola 3 (actual)" + briefs wave3-executor-1/2.

**Veredicto: APPROVED** — checklist completo en verde. Desviación del seed
aprobada con prueba matemática; observaciones no bloqueantes al final (con owner).

---

## 1. Integridad de integración

| Check | Evidencia | Resultado |
|---|---|---|
| Worktrees mergeados en orden del plan | `git log`: `446ded2 merge: wave 3 wave3-executor-1` → `a77a805 merge: wave 3 wave3-executor-2` → `2739473 docs: ola 3 integrada`. Base común `c05c6c4` (plan ola 3) | ✅ |
| Contenido de main == ramas de executors | `git diff wave3-executor-1 main -- backend/ .env.example docs/` → **vacío**; `git diff wave3-executor-2 main -- frontend/` → **vacío**. El único commit extra es `2739473` (1 archivo: `.workflow/plan.md`, estado de la tabla — tarea del integrador) | ✅ |
| `git status` limpio | `nothing to commit, working tree clean`, `main` up to date con `origin/main` | ✅ |
| Stashes | `git stash list` → **vacío** (E1 de ola 2 sigue saldada) | ✅ |
| Diff vs plan — executor-1 solo su mapa | e1: `d7ee5ce` (seed.js, models/cliente.rs, routes/cliente.rs, docs/API.md) + `185b3cc` (.env.example, backend/Cargo.toml, main.rs, otp.rs, routes/mod.rs, routes/verificacion.rs, docs/API.md). Todos en su mapa (`backend/Cargo.toml`, `backend/src/**`, `backend/scripts/**`, `docs/API.md`, `.env.example`) | ✅ |
| Diff vs plan — executor-2 solo su mapa | e2: `aadcb6e` (frontend/src/api.rs, frontend/src/components/alta_cliente.rs). NO tocó `frontend/Cargo.toml` (cero deps, como prohibía el brief), ni `tailwind.sh`, `Dioxus.toml`, `frontend/AGENTS.md` | ✅ |
| Nada fuera del mapa combinado | `git diff --name-only c05c6c4 2739473` → 13 archivos: los 12 de los mapas + `.workflow/plan.md` (planner/integrator). Cero archivos prohibidos | ✅ |

## 2. Build & tests (árbol integrado, corridos por el auditor)

| Check | Evidencia | Resultado |
|---|---|---|
| `cd backend && cargo build && cargo test` | `Finished dev profile` OK; `test result: ok. 32 passed; 0 failed` — 23 de ola 2 intactos + 9 netos nuevos (CURP dv, correo, fechas bisiesto/incoherentes, defaults serde de docs previos). Coincide con lo reportado por el integrador | ✅ |
| `cd frontend && cargo check --target wasm32-unknown-unknown` | `Finished dev profile` OK (exit 0) | ✅ |
| `cd frontend && cargo test` | `test result: ok. 14 passed; 0 failed` — los 11 de ola 2 + 3 nuevos: `solicitar_verificacion_construye_post_con_curp_y_telefono`, `confirmar_verificacion_incluye_codigo_en_el_body`, `telefono_verificado_true_solo_con_el_campo_en_true` (exactamente la lógica pura que el brief pedía) | ✅ |
| Verify de cada brief EN EL ÁRBOL INTEGRADO | Ambos verify (`backend: cargo build && cargo test`; `frontend: cargo check wasm + cargo test`) corridos sobre `main` @ `2739473`, no sobre worktrees | ✅ |

## 3. Audit gate del plan (evidencia de comandos)

| Check | Evidencia | Resultado |
|---|---|---|
| CURP robusta: dv malo → rechazada | Test `rechaza_formato_valido_con_digito_verificador_malo` (rechaza los 3 seed mutados + los originales 03/09) y en vivo: `POST /api/clientes` con `GACM940101HDFRRR07` (formato OK, dv real=9) → `"CURP inválida…dígito verificador correctos"`; con `GACM940101HDFRRR09` → success | ✅ |
| Algoritmo del dv: verificación INDEPENDIENTE de la fuente | `webfetch` de la fuente citada (curp.readthedocs.io, reproducción del Instructivo RENAPO DOF 18-10-2021): base36 con +1 si >N, pesos 18..1, suma ≡ 0 mod 10 — **matemáticamente equivalente** a `valor_curp`+`digito_verificador` de `cliente.rs` (el 18º con peso 1 ⇔ `(10 - suma17%10)%10`). El ejemplo oficial `SABC560626MDFLRN01` (suma 610) verifica en mi script externo y en el test del repo | ✅ |
| CURPs del seed | Script externo independiente: viejas `…ZR03`/`…RN09` → dv real 5/2 (**INVÁLIDAS** — el premise del brief era imposible); nuevas `…ZR05`/`…RN02` → **VÁLIDAS**; `GAML…R05` sin cambio → válida. Coincide exacto con los comentarios de los tests y con el decision log 2026-08-31 | ✅ (desviación aprobada, ver D1) |
| OTP NUNCA en claro en DB | `rg "codigo_hash" backend/src/` → 3 hits en `verificacion.rs` (insert solo hash; lectura del hash para comparar). **En vivo (Atlas, DB real del backend):** `verificaciones` contiene SOLO `{curp, telefono, codigo_hash, expira_en}` — campo `codigo` inexistente en los docs; `codigo_hash` del desafío pendiente **== sha256 del código impreso en el log** (`070091` → `aa3763e3…867b`) | ✅ |
| Sin token → 401 en solicitar/confirmar | curl sin Authorization: `solicitar: 401`, `confirmar: 401` (extraí­do `EmpresaSession` protege; rutas registradas dentro del router con `with_state`, `main.rs` L38–39) | ✅ |
| Código inválido → 400 | `confirmar` con `000000` (desafío vigente) → HTTP 400 `{"status":"error","message":"Código inválido o expirado"}` | ✅ |
| Sin desafío → 404 | `confirmar` con telefono sin desafío → HTTP 404 "No hay un código de verificación solicitado"; con curp sin cliente → HTTP 404 "Cliente no existe en la red PYMZA" | ✅ |
| Flujo completo + persistencia | solicitar → `{"status":"success"}` + `OTP MOCK para 5512345678: 226224` en el log; confirmar código correcto → HTTP 200 `{"status":"success","telefono_verificado":true}`; `GET /api/clientes/GACM940101HDFRRR09` → `telefono_verificado = True`; re-confirmar → 404 (desafío consumido) | ✅ |
| Expiración 10 min | `EXPIRA_MINUTOS=10`, chequeo `expira_en < now` → `codigo_invalido()` + limpieza (verificacion.rs L92–96). Comparación leída con el mismo `get_i64` que el flujo exitoso usa en vivo | ✅ (código) |
| Cero secretos commiteados | `rg "WHATSAPP_"` en AGENTS.md README.md docs/ backend/src/ → solo claves de env leídas en runtime (`otp.rs`), docs (mención de nombres) y `.env.example` con placeholders vacíos. `backend/.env` NO rastreado (`git ls-files backend/.env` → 0). Mock activo en la corrida (env vacía → filtrada por `env_no_vacia`) | ✅ |
| Rutas viejas `:empresa` | 0 strings de ruta con `:empresa`; los hits del patrón suelto son identificadores Rust (`routes::empresa`, `models::empresa`) — misma nota que ola 2. Rutas nuevas: `/api/verificaciones/solicitar`, `/api/verificaciones/confirmar` | ✅ |

## 4. Disciplina ponytail (scope)

| Check | Evidencia | Resultado |
|---|---|---|
| Deps = SOLO las 3 permitidas | `git show 185b3cc -- backend/Cargo.toml`: `reqwest 0.11` (default-tls, coherente con openssl del driver mongo — comentario en el diff), `rand 0.8`, `sha2 0.10`. Nada más. `frontend/Cargo.toml`: **cero** cambios | ✅ |
| Sin abstracciones no pedidas | Backend: `otp.rs` (137 l) y `verificacion.rs` (138 l) son los 2 archivos que el plan ordena; `OtpSender` trait + 2 impls es lo que el brief define. Frontend: sección condicional dentro de `alta_cliente.rs` (sin wizard), helpers planos en `api.rs` siguiendo el patrón `authed_request`/`sesion_ok` existente | ✅ |
| Menor diff | Ola completa: 13 archivos, ~+780/−25. Los 4 archivos nuevos son los pedidos (otp.rs, verificacion.rs, y el bloque del panel). Wiring de main.rs: +4 líneas (mod + 2 rutas) — no se amplió el State (ver D2) | ✅ |
| `ponytail:` con techo | `otp.rs` L64: `reqwest::Client` por llamada, techo "reutilizar si escala a miles/minuto". `otp.rs` L93: global `OnceLock` en vez de `AppState`, techo "mover a AppState si hay tests multi-config". Ambos nombran upgrade path | ✅ |
| CSS en sync sin regenerar | El brief pedía reusar clases: verificado que TODAS las clases usadas existen en el `assets/tailwind.css` commiteado, incluidas las escapadas `dark\:text-green-400` y `focus\:border-blue-500` (grep con prefijos escapados → 1 hit cada una). Por eso no hubo commit de CSS — correcto, no un olvido | ✅ |

## 5. Seguridad

| Check | Evidencia | Resultado |
|---|---|---|
| Inputs en trust boundary | `crear_cliente`: CURP robusta (formato+dv+calendario) y correo con `es_correo_valido` ANTES de tocar DB (en vivo: `"Correo inválido"` con CURP válida). `verificacion.rs`: body tipado por serde (`SolicitarVerificacionReq`/`ConfirmarVerificacionReq`); comparación solo de hashes; update `$set` de un campo con filtro exacto por curp | ✅ |
| Sin secretos commiteados | §3 arriba; el WhatsApp sender toma token de env en runtime, nunca lo loguea; fallos de envío → eprintln sin panickear (otp.rs L80–86) | ✅ |
| Licencias de deps nuevas | `reqwest`/`rand`/`sha2` = MIT OR Apache-2.0 — compatibles con `LICENSE-SOFTWARE` (existe en el repo) | ✅ |
| Aislamiento de tenant | Las rutas nuevas exigen JWT (`EmpresaSession`); la verificación es global por curp — coherente con el modelo de red colaborativa existente (`GET /api/clientes/:curp` ya es network-wide); el tenant del token no se usa para escribir nada ajeno | ✅ |

## Desviaciones y decisiones

| ID | Desviación | Evidencia | Estado |
|---|---|---|---|
| D1 | Seed: CURPs `…ZR03`/`…RN09` → `…ZR05`/`…RN02` (el brief pedía que las originales siguieran válidas) | Las originales son matemáticamente inválidas con el algoritmo oficial (dv real 5 y 2; script independiente + fuente DOF). Alternativa era implementar un algoritmo falso. Documentada en decision log 2026-08-31 ANTES de esta auditoría; tests del repo documentan los dv reales | **Aprobada** |
| D2 | `OtpSender` por global `OnceLock` en vez de ampliar el State a `AppState` (el brief permitía ambos, pidiendo `ponytail:` si se simplificaba) | `otp.rs` L93–95 nombra el techo. Evitó tocar el wiring de 10 rutas existentes | Aprobada (comentada) |

## Observaciones no bloqueantes (owners)

| ID | Observación | Owner / acción |
|---|---|---|
| O1 | Hash demo roto (preexistente, fuera de la ola 3): `demo@pymza.mx / demo123` → "Credenciales inválidas" contra Atlas, siendo el hash de DB idéntico al de `seed.js` (que dice `HASH_DEMO123`). `auth.rs` no fue tocado en esta ola y sus tests (round-trip argon2) pasan → el código de verificación funciona; el hash sembrado en la DB real no cuadra con la contraseña documentada. Bloquea el flujo demo documentado en AGENTS.md, no la ola 4 | **V**: re-seed (regenerar el hash con la contraseña demo real) o corregir la contraseña documentada |
| O2 | Desafíos OTP expirados quedan en `verificaciones` hasta que un `solicitar` los reemplace o un `confirmar` los limpie (no hay TTL index). Crecimiento ínfimo (1 doc por solicitud) | **Planner**: añadir `ponytail:`/TTL index (`expireAfterSeconds`) cuando haya proveedor real de WhatsApp, o en ola 5 (producción) |
| O3 | Datos de humo del INTEGRADOR quedaron en Atlas (empresa `nueva@empresa.mx`, cliente "Prueba OTP", desafío `GAML…` con teléfono de prueba). Los artefactos del auditor (empresa `auditor-w3@test.mx`, cliente `GACM…09`, su desafío) fueron ELIMINADOS al cierre: `deleteMany` exactos → 1 empresa, 1 cliente, 1 verificación | **V**: decidir si limpiar los del integrador o conservarlos como evidencia |
| O4 | `solicitar` no valida que el cliente exista (crea desafíos para curps inexistentes). Inofensivo: `confirmar` exige cliente + código correcto. No merece código extra ahora | Ninguna (nota de diseño) |

## Veredicto

- [x] **APPROVED** — la ola 4 (KYC/OCR + score por recibos) puede empezar.
- [ ] APPROVED WITH EXCEPTIONS
- [ ] REJECTED

Rolling plan: el planner re-planifica la ola 4 desde este estado; ninguna excepción
bloqueante pendiente. O1 requiere acción de V en paralelo (no gatea la ola 4).
