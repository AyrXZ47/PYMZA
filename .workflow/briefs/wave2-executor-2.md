# Brief: Wave 2 · Executor 2 — Docs: AGENTS.md, README, ROADMAP, API al estado real

> Copia de `.workflow/briefs/_template.md`. El executor no toca archivos que no
> posee, ni "obviamente". Desviaciones → decision log en `.workflow/plan.md`.

## Tarea

Refrescar la documentación al estado real post-ola 1, saldando la excepción E1
de la auditoría ola 1 (`.workflow/audits/wave1.md`): `AGENTS.md` quedó FALSO
("token is still static token-temporal-123 (no JWT, routes don't validate
it)"). Verifica contra el código real (lee los archivos listados en "Read
first") y corrige SOLO lo que sea falso o esté desactualizado:

1. **`AGENTS.md` (raíz)**:
   - Sección "Login" del bloque Gotchas → el auth ahora es **JWT real**
     (HS256, `jsonwebtoken` v9, secreto en env `JWT_SECRET` obligatoria, exp
     24h, extractor `EmpresaSession`; 8 rutas protegidas con 401; las únicas
     públicas son `POST /api/login` y `POST /api/empresas`).
   - Tabla "API Endpoints" → actualizar: `GET /api/creditos` y
     `GET /api/dashboard` ya NO llevan `/:empresa` (la empresa sale del JWT);
     indicar que el resto de rutas requieren `Authorization: Bearer <jwt>`.
   - Sección "Structure" → backend ya está modularizado (`main.rs` = wiring +
     `routes/`, `models/`, `auth.rs`); frontend tiene `api.rs` +
     `components/` (ya no "todo en main.rs").
   - Sección "Secrets / Config" → añadir `JWT_SECRET=""` al `.env.example`
     mencionado y la nota de que el backend falla al arrancar si falta.
   - No tocar: reglas del workflow, RTK, skills, stack, estructura general de
     la sección.
2. **`README.md`**:
   - Actualizar estructura backend/frontend (módulos, auth.rs).
   - Resumen de endpoints sin `/:empresa`.
   - Nota breve de autenticación JWT (Bearer).
3. **`docs/ROADMAP.md`**: marcar como HECHO lo de la ola 1 (JWT real,
   multi-tenant con tenant = correo + script de migración idempotente,
   frontend modularizado, registro de empresa con auto-login si aplica tras la
   ola 2), mantener pendientes (landing → en curso ola 2; inversores, soporte,
   FICO/CdC, Stripe, contratos PDF, OCR real, KYC, despliegue). Corregir las
   ramas "sin merge" (todas están mergeadas ya) y referenciar
   `.workflow/audits/wave1.md`.
4. **`docs/API.md`**: ya fue actualizada al contrato ola 1 por el executor-1 de
   la ola 1 (commit 28e0510). NO reescribir: solo verificar con `rg` que no
   queden restos de `token-temporal-123` ni `/:empresa`; si aparece alguno,
   corregir el caso puntual y reportarlo.

NO toques `frontend/AGENTS.md` (es la referencia de la API de Dioxus 0.7).

## Definition of done

- `rg "token-temporal-123" AGENTS.md README.md docs/` → 0 hits (E1 saldada).
- `rg ":empresa" AGENTS.md README.md docs/` → 0 hits (rutas viejas muertas).
- `AGENTS.md` describe correctamente: JWT real + `JWT_SECRET` + 8 rutas
  protegidas + tenant = correo en endpoints.
- `README.md` y `docs/ROADMAP.md` consistentes con el código actual.
- El comando de verify de abajo pasa.

## Files you own

- `AGENTS.md` (raíz)
- `README.md`
- `docs/ROADMAP.md`
- `docs/API.md`

## Files forbidden

- `frontend/**` (incl. `frontend/AGENTS.md`), `backend/**`, `PYMZA.md`
- `docs/INVESTIGACION.md`, `docker-compose.yml`, `Dockerfile.*`
- `.workflow/**`, `skills/**`, `.env.*`

## Read first

- `.workflow/plan.md` — tabla de olas y "Ola 2 (actual)" (contexto).
- `.workflow/audits/wave1.md` — hallazgos E1/E2 y evidencia del contrato real.
- `backend/src/main.rs` (wiring de rutas), `backend/src/auth.rs` (extractor
  JWT), `backend/src/routes/credito.rs` (no `:empresa`).
- `frontend/src/api.rs` (API_BASE, sesión localStorage) — solo lectura.

## Verify command

```bash
! rg "token-temporal-123|/api/creditos/:empresa|/api/dashboard/:empresa" AGENTS.md README.md docs/ && rg -q "JWT|jwt" AGENTS.md README.md docs/API.md && echo "docs OK"
```

## Commit

- MANDATORY: conventional commits, resumen corto, imperativo, una línea
  (`docs:` aquí). <72 chars. Sin atribución de IA, sin trailers.
- Un cambio lógico por commit (sugerencia: `docs: estado real post-ola 1
  (jwt, multi-tenant)`).
- Commitea SOLO tus archivos poseídos.
- BRANCH ISOLATION (obligatorio): commitea y pushea SOLO a tu rama de
  worktree — `git push origin <tu-rama>` — tras cada commit. Nunca a `main`
  ni a otra rama; nunca merge/rebase/fast-forward de ramas ajenas.

## Report back

- Archivos cambiados, salida del verify, y cualquier resto de
  `token-temporal-123`/`:empresa` que encontraras en `docs/API.md` (y cómo lo
  corregiste).