# Brief: Wave 5 · Executor 1 — Backend: OCR real (tesseract), KYC de INE y score por recibos

> Copia de `.workflow/briefs/_template.md`. El executor no toca archivos que no
> posee, ni "obviamente". Desviaciones → decision log en `.workflow/plan.md`.

## Tarea

Implementar EXACTAMENTE el "Contrato API ola 5" de `.workflow/plan.md`:

1. **`backend/src/ocr.rs`** (nuevo):
   - `extraer_texto(bytes: &[u8], mime: &str) -> Result<String, OcrError>`:
     escribe a archivo temporal (extensión según mime: png/jpg/webp), corre
     `tesseract <tmp> stdout -l $OCR_LANG (default "spa") --psm 6` con
     `tokio::process::Command` + `tokio::time::timeout(30s)`, borra el temp.
     Si el binario `tesseract` no existe en PATH o falla →
     `OcrError::NoDisponible` (el handler lo mapea a 500 con mensaje claro,
     sin panic). TESSDATA: si el entorno lo necesita, respeta env
     `TESSDATA_PREFIX` (tesseract lo usa nativamente; no implementes nada).
   - `buscar_curp(texto: &str) -> Option<String>` — regex del formato CURP
     sobre el texto (tolerante al ruido de OCR: espacios/ligaduras entre
     caracteres). Función pura, testeada con texto de OCR simulado (ruido,
     saltos de línea, caracteres mal leídos) y con el texto exacto de la CURP.
   - `buscar_monto(texto: &str) -> Option<f64>` — regex de montos
     ($1,234.56, 1234.56 MXN, TOTAL…). Función pura, testeadas.
2. **Endpoints** (en `routes/cliente.rs` o `routes/kyc.rs` nuevo — tu
   elección, documenta con `ponytail:`):
   - `POST /api/clientes/:curp/kyc` (protegido): body
     `{archivo_b64, mime}`. Validar en orden: mime ∈ {image/png,
     image/jpeg, image/webp} → 400; base64 válido → 400; tamaño
     decodificado ≤ 2 MB → 400 (ANTES de decodificar, del largo del string
     b64); cliente existe → 404. Corre OCR → `buscar_curp` → compara con el
     curp del path → si coincide, update `ine_verificada: true`. Respuesta
     exacta del contrato. Sin CURP en el texto → success con
     `curp_ine: null, coincide: false` + message.
   - `POST /api/clientes/:curp/recibos` (protegido): body
     `{archivo_b64, mime, tipo}` — tipo ∈ {luz, agua, telefono} → 400.
     Misma validación de archivo. OCR → `buscar_monto` → si legible (monto
     Some O texto ≥50 chars): count recibos del cliente (colección `recibos`,
     query por curp) — si ya hay 2 → 400 "Máximo 2 recibos por cliente".
     Inserta `{curp, empresa: sesion.correo, tipo, monto_leido, fecha}`.
     Score: `score += 25` y `nivel_riesgo = nivel_por_score(score)` —
     función pura: `>=750 "Bajo"`, `>=550 "Medio"`, `<550 "Alto"` (tests,
     umbrales del contrato). Update del cliente. Respuesta exacta del
     contrato (`monto_leido, score, nivel_riesgo, recibos_contados`).
3. **`Cliente`**: campo `ine_verificada: bool` con
   `#[serde(default, skip_serializing_if...)]`? — NO skip: el frontend necesita
   leerlo siempre; usa `#[serde(default)]` solamente.
4. **Deps: SOLO `base64 = "0.22"`.** El motor OCR es el binario tesseract —
   CERO crates de OCR.
5. **Fixture**: `backend/scripts/fixture_ine.png` — PNG ~600×400 con texto
   grande legible: una CURP válida del seed (p. ej. `GAML930528HDFLNR05`) y
   un nombre (`MARIA GOMEZ LOPEZ` o similar). Genera la imagen con la
   herramienta disponible (ImageMagick `convert`, PIL de python, o un PNG
   dibujado a mano) — debe ser legible por `tesseract --psm 6` (verifícalo
   corriendo tesseract sobre tu fixture antes de commitearlo).
6. **`seed.js`**: reemplazar el hash precomputado de `demo123` por el de
   `demo1234` (argon2id PHC — genera el hash con un pequeño test de
   `hashear_password` o similar; NO commitees la contraseña en claro más que
   el comentario/documentación ya existente: la demo es pública de propósito,
   ver plan). Ajusta el print final.
7. **docs/API.md**: los 2 endpoints nuevos + campo nuevo del cliente +
   `OCR_LANG`. **.env.example**: `OCR_LANG=""`.
8. Tests: `buscar_curp` (texto limpio y ruidoso), `buscar_monto`,
   `nivel_por_score` (3 umbrales + bordes), validaciones (mime, tamaño por
   largo b64, base64 inválido) — las funciones puras sin DB. Los 46 tests
   existentes deben seguir pasando.

Ponytail: sin abstracciones de proveedores de OCR ni pipeline de imágenes —
un binario, un archivo temporal, dos regex. Marca con `ponytail:` los
atajos nombrando su techo.

## Definition of done

- KYC con fixture → `coincide: true` y `ine_verificada` persiste; con CURP
  distinta → `coincide: false` sin marcar.
- mime inválido → 400; >2MB (por largo b64) → 400; b64 inválido → 400.
- Recibo legible → +25 con nivel recalculado; 3er recibo → 400.
- tesseract ausente → 500 con mensaje claro (simulable con PATH vacío).
- La imagen no se persiste en ninguna colección.
- El comando de verify de abajo pasa (tests viejos + nuevos).

## Files you own

- `backend/Cargo.toml` (solo la dep `base64`)
- `backend/src/**`
- `backend/scripts/**` (fixture + seed)
- `docs/API.md`
- `.env.example`

## Files forbidden

- `frontend/**`, `AGENTS.md` (raíz), `README.md`, `docs/ROADMAP.md`,
  `docs/INVESTIGACION.md`, `PYMZA.md`, `docker-compose.yml`, `Dockerfile.*`
  (tesseract en Docker es tarea de la ola 6), `.workflow/**`, `skills/**`,
  `backend/.env`

## Read first

- `.workflow/plan.md` — sección "Ola 5 (actual)" y Contrato API (shapes
  exactas de respuesta).
- `backend/src/routes/cliente.rs` (patrón de handlers + validaciones),
  `backend/src/models/cliente.rs` (Cliente + defaults serde ola 3),
  `backend/src/main.rs` (wiring de rutas), `backend/src/auth.rs`
  (`EmpresaSession`, `es_correo_valido`), `backend/scripts/seed.js`.
- `backend/src/otp.rs` — patrón de módulo propio con funciones puras + tests.

## Verify command

```bash
cd backend && cargo build && cargo test
```

(Extra recomendado antes de commitear el fixture:
`PATH=$PATH tesseract backend/scripts/fixture_ine.png stdout -l spa --psm 6`.)

## Commit

- MANDATORY: conventional commits, resumen corto, imperativo, una línea
  (`feat:`, `fix:`, `chore:`, `docs:`, `refactor:`, `test:`, `perf:`,
  `style:`, `build:`, `ci:`, `revert:`, scope opcional). <72 chars. Sin
  atribución de IA, sin trailers.
- Un cambio lógico por commit (sugerencia: `feat(backend): motor ocr
  tesseract con regex curp y monto`, `feat(backend): kyc de ine y score por
  recibos`, `fix(backend): hash demo1234 en seed`).
- Commitea SOLO tus archivos poseídos (el fixture es binario — commit normal).
- BRANCH ISOLATION (obligatorio): commitea y pushea SOLO a tu rama de
  worktree — `git push origin <tu-rama>` — tras cada commit. Nunca a `main`
  ni a otra rama; nunca merge/rebase/fast-forward de ramas ajenas.

## Report back

- Archivos creados/cambiados, salida de `cargo test` (número), output del
  tesseract sobre tu fixture (evidencia de legibilidad), decisiones tomadas
  (ubicación de handlers, regex usadas) y desviaciones del contrato.