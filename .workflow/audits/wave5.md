# Auditoría Ola 5 — KYC/OCR real + score alternativo por recibos

- **Fecha:** 2026-09-05 (sesión de auditoría en fresco, árbol integrado `main` @ `91de796`)
- **Alcance:** `.workflow/plan.md` sección "Ola 5", briefs `wave5-executor-{1,2}.md`, `.workflow/audit-checklist.md`
- **Veredicto: APPROVED WITH EXCEPTIONS** (E1, E2 — menores, con owner; no bloquean la ola 6)

---

## 1. Integridad de la integración

| Check | Evidencia | OK |
|---|---|---|
| Worktrees mergeados a main | `git log --oneline main..wave5-executor-1` y `main..wave5-executor-2` → **vacíos** (0 commits sin mergear) | ✅ |
| `git status` limpio | `git status --porcelain` → sin salida; `git stash list` → vacío | ✅ |
| Nada fuera del mapa de propiedad | `git show --stat` por commit de la ola 5: EJ-1 tocó `backend/Cargo.toml`, `backend/src/**` (main.rs, ocr.rs, models/cliente.rs, routes/{cliente,kyc,mod}.rs), `backend/scripts/**` (fixture_ine.png, seed.js), `docs/API.md`, `.env.example`; EJ-2 tocó `frontend/Cargo.toml`, `frontend/src/**` (api.rs, components/alta_cliente.rs), `frontend/assets/tailwind.css`. Ambos 100% dentro de su territorio | ✅ |
| Todo lo planeado presente | ocr.rs (342 l), routes/kyc.rs (310 l), 2 rutas en main.rs, `ine_verificada` con `#[serde(default)]` sin skip, fixture_ine.png (13.5 KB), seed.js hash demo1234, docs/API.md +139, `.env.example` OCR_LANG, frontend api.rs +378 / alta_cliente.rs +228, CSS regenerado | ✅ |

Commits de la ola: EJ-1 `2ecb482, 5ea6a18, 446d720, 0281262, b5f39aa, 8003813`; EJ-2 `83662bd, 0b68b24`; merges `9603dd1, e4fd121`; docs integrador `91de796`.

## 2. Build y tests (árbol integrado)

| Comando | Salida | OK |
|---|---|---|
| `cd backend && cargo build && cargo test` | `Finished dev profile` → `test result: ok. 60 passed; 0 failed` (46 previos + 14 nuevos: parsers, umbrales, validaciones) | ✅ |
| `cd frontend && cargo check --target wasm32-unknown-unknown && cargo test` | `Finished dev profile` (wasm) → `test result: ok. 36 passed; 0 failed` (22 previos + 14 nuevos) | ✅ |
| Verify completo EJ-2 (`./tailwind.sh` + sync CSS) | `git status --porcelain -- assets/tailwind.css` → sin salida = CSS en sync | ✅ |

## 3. Disciplina ponytail

| Check | Evidencia | OK |
|---|---|---|
| Cero deps nuevas salvo `base64` | `git show` en ambos Cargo.toml: solo `base64 = "0.22"` (backend y frontend). Cero crates de OCR (motor = binario tesseract, dec. log 2026-09-04) | ✅ |
| Sin abstracciones no pedidas | Módulo `kyc.rs` justificado con `ponytail:` (L3: comparte validación de archivo; techo: separar por endpoint). `ocr.rs` usa tempfile manual (pid+contador) en vez de crate tempfiles — documentado en doc-comment | ✅ |
| Diff mínimo | Backend ~810 l (de ellas ~450 tests), frontend ~645 l. Placeholder OCR viejo (`/api/ocr`) intacto — fuera de alcance, correcto | ✅ |
| `ponytail:` con techo | `kyc.rs` L3; `ocr.rs` L44-46 nombra el techo del tempfile; `buscar_nombre` L220-222 techo "RENAPO/proveedor KYC (ola 7)" | ✅ |

## 4. Seguridad

| Check | Evidencia | OK |
|---|---|---|
| Trust boundary validada en orden | `validar_archivo`: mime → tamaño (por LARGO b64, ANTES de decodificar) → base64 → 404. Evidencia en vivo abajo | ✅ |
| Sin secretos commiteados | `git ls-files` → solo `.env.example` (placeholders vacíos, incluido `OCR_LANG=""`); `backend/.env` gitignored (`git check-ignore` OK). Diff de la ola sin tokens/URIs (hash argon2 demo es intencional y público por diseño) | ✅ |
| La imagen NO se persiste | Código: temp borrado en TODOS los caminos (`ocr.rs` L87, incl. timeout y error); en vivo: `db.getCollectionNames()` → `recibos, clientes, empresas, verificaciones, dashboard_stats, planes_pago` — sin colección de imágenes | ✅ |
| Tenant scoping | `recibos` docs: `empresa` = correo del token (`sesion.correo`), jamás del body; tope de 2 GLOBAL por curp (decisión documentada en código y API.md). El tenant (curp → cliente) sale del path pero el cliente es de la red compartida — conforme al modelo de red del plan | ✅ |
| Release gate `skills/security-audit` | Corresponde a la **ola 6** (producción), no aquí | n/a |

## 5. Audit gate de la ola 5 (evidencia en vivo, humo contra DB local)

Entorno: mongod local (`--dbpath /tmp/opencode/mongo-data`, puerto 27017), `mongosh < backend/scripts/seed.js` fresco, backend `BIND_ADDR=127.0.0.1:3999` con `MONGODB_URI` local y `JWT_SECRET` efímera (env del proceso; `backend/.env` de Atlas de V sin tocar). Cero efectos en producción. Entorno limpiado al final (mongod shutdown + dbpath borrado).

| Check del plan | Resultado | OK |
|---|---|---|
| `seed.js` con hash demo1234 | `POST /api/login` demo@pymza.mx/demo1234 sobre seed fresco → JWT emitido | ✅ |
| KYC fixture → coincide y persiste | `POST .../GAML930528HDFLNR05/kyc` con fixture → `{curp_ine: "GAML930528HDFLNR05", nombre_ine: "MARIA GOMEZ LOPEZ", coincide: true, ine_verificada: true}`; GET posterior → `ine_verificada: true` | ✅ |
| KYC con CURP distinta NO marca | fixture contra path `RAMJ920215MDFMZR05` → `coincide: false`, `ine_verificada: false` + message; GET posterior → `false` | ✅ |
| mime inválido → 400 | `text/plain` → 400 "Mime no permitido…" | ✅ |
| b64 inválido → 400 | `no-es-b64!!` → 400 "Base64 inválido" | ✅ |
| tipo inválido → 400 | `tipo: "gas"` → 400 "Tipo inválido…" | ✅ |
| **>2MB → 400 (contrato)** | **→ HTTP 413** "Failed to buffer the request body: length limit exceeded" (PNG real 2.4 MB → b64 3.2 MB). Ver **E1** | ⚠️ |
| Recibo legible → +25 y nivel | con imagen sintética legible (74 chars OCR, `TOTAL: $450.00 MXN`): 640→665 (Medio) → 690 (Medio, correcto: <750). `monto_leido: 450.0`, `recibos_contados: 1, 2` | ✅ |
| 3er recibo → 400 | → 400 "Máximo 2 recibos por cliente"; `db.recibos.count()` quedó en 2 (el rechazado no inserta) | ✅ |
| `buscar_curp`/`buscar_monto` con ruido | `cargo test` 60 OK (incluye ruido, salto de línea, minúsculas, ligaduras, bordes 749/750/549/550, fechas/folios no confunden) | ✅ |
| Tesseract ausente → 500 claro | backend relanzado con `PATH=/nonexistent` → 500 `{"status":"error","message":"OCR no disponible en este servidor"}`, sin panic | ✅ |
| Imagen no persistida | ver colecciones en §4 | ✅ |
| Cero deps nuevas salvo base64 | ver §3 | ✅ |
| Humo UI (navegador) | Pendiente del humano (V), igual que en olas previas. wasm check + tests de los componentes cubren la lógica | ⏳ owner V |

**Nota metodológica del humo de recibos:** el plan preveía probar recibos con el fixture, pero el fixture produce 40 chars OCR (<50, sin monto) → `legible=false`. El flujo legible/tope se probó con imagen sintética propia (PIL, /tmp) — el CÓDIGO queda verificado end-to-end; lo que no cumple es el RECURSO de humo. Ver **E2**.

## Hallazgos y excepciones

### E1 — `>2MB` devuelve 413, no el 400 del contrato (menor, owner: planner ola 6)
El default body-limit de Axum (2 MB del body JSON) rechaza la petición **antes** del handler: un archivo >~1.5 MB decodificados (b64 >2 MB) → HTTP 413 sin el JSON de error del contrato. Efecto de seguridad equivalente (rechazo) y el límite del handler sigue como defensa en profundidad, pero el código HTTP y el mensaje no son los del contrato ("El archivo excede el máximo de 2 MB", 400). El check unitario `b64_excede_maximo` es inalcanzable por HTTP hoy. **Propuesta (ola 6 ya toca tower-http/rate-limit):** `DefaultBodyLimit::max(~3 MB)` en el Router o documentar el 413 en API.md. No bloquea.

### E2 — `fixture_ine.png` no sirve para el humo de recibos (menor, owner: planner ola 6)
OCR del fixture = 40 chars (<50) y sin monto → el humo planeado "recibos dos veces para ver el tope" con el fixture no aplica bonus ni tope (confirmado: `tesseract fixture_ine.png → 40 chars`). El flujo real quedó verificado en vivo con imagen sintética (§5). **Propuesta:** regenerar el fixture con más texto o añadir `fixture_recibo.png` cuando la ola 6 arme el Docker del humo. No bloquea.

### N1 — Imagen corrupta → 500 "OCR no disponible" (informativo)
PNG/truncado inválido (bytes aleatorios): tesseract falla → 500 con mensaje que sugiere servidor roto cuando el problema es el archivo del cliente. Conforme al contrato literal (fallo del binario → 500). Techo: validar magic bytes png/jpeg/webp en `validar_archivo` (→ 400 "imagen inválida") si el soporte lo pide.

### N2 — Tope de recibos sin atomicidad (informativo)
`count_documents` + `insert_one` no son atómicos: dos requests simultáneos podrían dejar 4 recibos. Volumen ínfimo hoy (demo, un tenant). Upgrade path si hay concurrencia real: unique index parcial por (curp, tipo) o transacción.

## Veredicto

**APPROVED WITH EXCEPTIONS** — E1 y E2 son menores, documentados, con owner (planner de la ola 6) y sin riesgo de seguridad (todo rechazo >2MB termina rechazado; el flujo de recibos está verificado en vivo). La ola 6 (Railway, CORS productivo, rate limiting, release gate `skills/security-audit`) puede arrancar; E1/E2 caen naturalmente en su alcance (tower-http, Docker del humo).

Pendiente pre-ola 6: humo UI en navegador (owner V, como en olas 2–4).
