# Brief: Wave 1 · Executor 1 — Backend: JWT real + aislamiento multi-tenant

> Copia de `.workflow/briefs/_template.md`. El executor no toca archivos que no
> posee, ni "obviamente". Desviaciones → decision log en `.workflow/plan.md`.

## Tarea

Reemplazar el token estático `token-temporal-123` por JWT real y aislar los
datos por empresa, implementando EXACTAMENTE el "Contrato API ola 1" de
`.workflow/plan.md`. En concreto:

1. Añadir `jsonwebtoken = "9"` a `backend/Cargo.toml` (única dep nueva
   permitida).
2. En `backend/src/auth.rs`: funciones de emisión y verificación de JWT
   (HS256, claims `sub=<correo>`, `nombre=<nombre_empresa>`, `exp=24h`) con
   `JWT_SECRET` desde env — si falta al arrancar, el backend debe fallar con
   mensaje claro. Un extractor tipo `EmpresaSession` (FromRequestParts de
   Axum 0.6) que lee `Authorization: Bearer`, valida y expone
   `correo`/`nombre`; 401 si falta/inválido/expirado.
3. `login_empresa` emite JWT real (la respuesta conserva la forma
   `{status, empresa, token}`).
4. Rutas: `GET /api/creditos` y `GET /api/dashboard` pierden el path param
   `:empresa` (empresa = token). `autorizar` y `reportar` pierden `empresa`
   del body (sale del token); actualizar `AutorizarReq`/`ReportarReq` en
   `models/`. Las 8 rutas protegidas del contrato usan el extractor;
   `/api/login` y `/api/empresas` quedan públicas.
5. Documentos nuevos en `planes_pago`/`dashboard_stats` guardan
   `empresa: <correo>`.
6. `backend/scripts/migrate_tenant.js`: idempotente; para cada doc de
   `empresas`, actualiza `planes_pago` y `dashboard_stats` donde
   `empresa == nombre_empresa` → `correo`. Actualiza también `seed.js` si el
   schema lo requiere.
7. Actualizar `docs/API.md` al contrato nuevo y `.env.example` (añadir
   `JWT_SECRET=""`).

Ponytail: nada de refresh tokens, roles, ni middleware genérico reutilizable —
solo lo del contrato. Marca con `ponytail:` los atajos deliberados nombrando
su techo. Deja tests de la lógica no trivial.

## Definition of done

- Login/alta emiten JWT real; `token-temporal-123` no aparece en `backend/`.
- Sin token o con token inválido/expirado → 401 en las 8 rutas protegidas.
- `/api/creditos` y `/api/dashboard` sin path param; `autorizar`/`reportar`
  sin `empresa` en body.
- Tests unitarios: round-trip emite→valida, token expirado rechazado, token
  malformado rechazado, token firmado con otro secreto rechazado.
- `cargo build && cargo test` pasa. El comando de verify de abajo pasa.

## Files you own

- `backend/Cargo.toml`
- `backend/src/**`
- `backend/scripts/**`
- `docs/API.md`
- `.env.example`

## Files forbidden

- `frontend/**` (el executor-2 lo adapta en paralelo contra el contrato)
- `backend/.env` (secreto del humano; ni leerlo en voz alta ni commitearlo)
- `docker-compose.yml`, `Dockerfile.*`, `README.md`, `docs/ROADMAP.md`,
  `PYMZA.md`, `AGENTS.md`, `.workflow/**`, `skills/**`

## Read first

- `AGENTS.md` (raíz) — stack, gotchas (OpenSSL, secretos, reglas).
- `.workflow/plan.md` — sección **Contrato API ola 1** (tu fuente de verdad).
- `backend/src/main.rs`, `backend/src/auth.rs`, `backend/src/routes/*.rs`,
  `backend/src/models/*.rs` — estado actual (10 rutas, token estático).
- `docs/API.md` — referencia actual de endpoints (la vas a reescribir).

## Verify command

```bash
cd backend && cargo build && cargo test
```

## Commit

- MANDATORY: conventional commits, resumen corto, imperativo, una línea
  (`feat:`, `fix:`, `chore:`, `docs:`, `refactor:`, `test:`, `perf:`,
  `style:`, `build:`, `ci:`, `revert:`, scope opcional). <72 chars. Sin
  atribución de IA, sin trailers.
- Un cambio lógico por commit. Un commit por tarea.
- Commitea SOLO tus archivos poseídos.
- BRANCH ISOLATION (obligatorio): commitea y pushea SOLO a tu rama de
  worktree — `git push origin <tu-rama>` — tras cada commit. Nunca a `main`
  ni a otra rama; nunca merge/rebase/fast-forward de ramas ajenas.
- Sugerencia: `feat(backend): auth JWT real y aislamiento multi-tenant`.

## Report back

- Archivos cambiados, salida de `cargo test`, desviaciones del contrato (si
  las hubo y por qué), y confirmación de que `JWT_SECRET` quedó solo en
  `.env.example` como placeholder vacío.
