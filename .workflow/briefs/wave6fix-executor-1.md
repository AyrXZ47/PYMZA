# Brief: Wave 6-fix · Executor 1 (único)

> Hotfix del release gate (ola 6 REJECTED). La auditoría YA validó todas las
> recetas de fix en vivo — aplica exactamente lo descrito aquí, no re-inventes.
> Nunca toques un archivo que no posees, ni "obviamente". Desviaciones →
> decision log de `.workflow/plan.md` y reporta.

## Task

Tres fixes pequeños, cada uno un commit (ver commits abajo):

- **T1 — F1/F2 (HIGH, DoS):** en `backend/src/routes/credito.rs`, `evaluar`
  acepta `plazo_meses` gigante (→ `collect()` de ~86 GB → OOM del proceso) y
  `autorizar` persiste el plan envenenado (cartera congelada, contrato = OOM
  persistente). Fix: validar en **AMBOS** handlers `plazo_meses ∈ 3..=12` y
  `monto > 0` finito → `400` con el mensaje de error del contrato (mismo
  estilo JSON `{"status":"error","message":...}` que ya usan). Payloads que
  DEBEN dar 400 tras el fix: `plazo_meses: 1000000`, `monto: -1`
  (`monto: 1e400` ya devuelve 400 hoy por el extractor — verifícalo). Añade
  tests: uno por payload rechazado en `evaluar` y en `autorizar` (aserta
  status 400 y que NADA se inserta en `planes_pago`).
- **T2 — S1 (bloqueante de Railway):** `Dockerfile.backend` no construye:
  (1) `COPY backend/Cargo.toml backend/Cargo.toml` deja el manifest en
  `/build/backend/`, no en `/build/` → `could not find Cargo.toml in /build`;
  (2) `FROM rust:1.83-bookworm` ya no compila `time-core-0.1.9` (edition2024).
  Fix validado por el auditor: `COPY backend/Cargo.toml ./Cargo.toml` +
  `COPY backend/src ./src` + `FROM rust:1.97-bookworm AS builder`. Conserva
  `tesseract-ocr` + `tesseract-ocr-spa` y el usuario no-root que ya tiene.
  NO cambies nada más del Dockerfile.
- **T3 — S2 (CSS):** el botón "Descargar contrato" usa 4 clases que faltan en
  el CSS compilado (`hover:bg-blue-700`, `py-1.5`, `hover:bg-slate-300`,
  `dark:bg-slate-700`). Regenera: `cd frontend && ./tailwind.sh` y commit del
  `frontend/assets/tailwind.css` resultante. NUNCA edites el CSS a mano.
- **T4 — F5 (APROBADO por V 2026-09-06):** carrera en
  `alta_empresa`: sin índice único, dos registros simultáneos comparten la
  tenant key. Fix: crear índice único en `empresas.correo` durante la
  inicialización del pool en `backend/src/db.rs` (maneja el error de duplicado
  si el índice ya existe). Un commit aparte.

## Definition of done

- `cargo test` backend: 68 previos + los tests nuevos de F1/F2, 0 failed.
- Los 3 payloads F1/F2 (contra el backend corriendo) devuelven **400** — ni
  413, ni 500, ni OOM.
- `docker compose build` construye AMBOS servicios sin error.
- `docker run --rm <imagen backend> tesseract --version` imprime `tesseract
  5.3.x` con `spa` en `tesseract --list-langs`.
- `grep -c "hover:bg-blue-700\|py-1.5\|hover:bg-slate-300\|dark:bg-slate-700"
  frontend/assets/tailwind.css` → los 4 matches presentes.
- (Si T4 aprobado) segundo `POST /api/empresas` con el mismo correo → 4xx con
  mensaje claro, y `db.empresas.getIndexes()` muestra el índice único.
- El verify command pasa.

## Files you own

- `backend/src/routes/credito.rs`
- `backend/src/db.rs` (SOLO si T4 fue aprobado)
- `Dockerfile.backend`
- `frontend/assets/tailwind.css` (regenerado por el script, nunca a mano)

## Files forbidden

- Todo lo demás. Especialmente: `frontend/src/**` (la causa S2 ya está
  commiteada; no la toques — solo regenera el CSS), `backend/Cargo.toml`
  (cero deps nuevas), `docs/**`, `docker-compose.yml`, `backend/src/db.rs`
  (si T4 NO fue aprobado), `.env*`, `.workflow/**`.

## Read first

- `.workflow/audits/wave6.md` — secciones 6 (F1/F2 con evidencia) y 7 (S1/S2
  con las recetas exactas ya validadas).
- `backend/src/routes/credito.rs` — handlers `evaluar` y `autorizar` + dónde
  viven sus tests inline (mira cómo la tabla de tasas 3m=3%…12m=15% ya acota
  el rango válido; reutiliza esa constante si existe).
- `Dockerfile.backend` actual y `Dockerfile.frontend` (el fix rust:1.97 ya se
  hizo ahí en `bd3553a` — copia el patrón).
- `docs/API.md` — mensajes de error del contrato para reutilizar el estilo.

## Verify command

```bash
cd backend && cargo test && docker compose build backend 2>&1 | tail -5 && docker run --rm $(docker images -q pymza-backend | head -1) tesseract --version
```

(Verificación de CSS aparte, una sola vez en T3:
`grep -c "hover:bg-blue-700" frontend/assets/tailwind.css` → ≥1.)

## Commit

- MANDATORY: conventional commits, corto, imperativo, una línea. Sin
  atribución AI ni trailers.
- Un cambio lógico por commit — aquí son 3 (o 4) commits:
  1. `fix(backend): valida plazo y monto en evaluar y autorizar`
  2. `fix(docker): corrige rutas copy y rust 1.97 en imagen backend`
  3. `chore(frontend): regenera css compilado`
  4. (T4 si aprobado) `fix(backend): indice unico en correo de empresa`
- Commit ONLY your owned files.
- BRANCH ISOLATION (mandatory): commit and push ONLY to your own worktree
  branch — `git push origin wave6fix-executor-1` — after each commit. Never
  push to `main` or another branch; never merge, rebase, or fast-forward
  anyone else's branch.

## Report back

- Archivos cambiados por commit, output de `cargo test` (número de tests),
  output del build Docker de ambos servicios, y output de los 3 payloads
  F1/F2 (400 esperado). Si T4: output del índice y del duplicado rechazado.
- Cualquier desviación de la receta (p. ej. si `3..=12` choca con algo) se
  reporta, NO se improvisa.
