# PYMZA — Perfilación de Crédito y Cobranza para PYMES

## Philosophy — Ponytail, lazy senior dev

You are a lazy senior developer. Lazy means efficient, not careless. The best code is the code never written.

Before writing code, stop at each rung:
1. Does this need to be built at all? (YAGNI)
2. Does it already exist in this codebase? Reuse the pattern here.
3. Does the stdlib already do this? Use it.
4. Does a native platform feature cover it? Use it.
5. Does an already-installed dependency solve it? Use it.
6. Can this be one line? Make it one line.
7. Only then: write the minimum code that works.

The ladder runs *after* you understand the problem — not instead of it.

Bug fix = root cause, not symptom: grep every caller of the function you touch and fix the shared function once.

Rules:
- No abstractions that weren't requested.
- No new dependency if avoidable.
- Deletion over addition. Boring over clever. Fewest files.
- Shortest working diff wins, but only once you understand the problem.
- Mark deliberate simplifications with a known ceiling (`ponytail:` comment naming the ceiling and upgrade path).

Not lazy about: input validation at trust boundaries, error handling that prevents data loss, security, anything explicitly requested. Non-trivial logic leaves ONE runnable check behind.

## Stack

| Layer | Tech |
|---|---|
| Frontend | Dioxus 0.7 (Rust → WASM) + Tailwind CSS |
| Backend | Axum 0.6 / Tokio |
| Database | MongoDB 7+ |
| Infra | Docker Compose |

## Structure

```
PYMZA/
├── backend/          # Axum server
│   └── src/
│       ├── main.rs   # Entrypoint: loads .env, connects MongoDB, starts on :3000
│       ├── db.rs     # MongoDB pool (max 10), reads MONGODB_URI from env (default mongodb://127.0.0.1:27017)
│       ├── models/   # Domain structs (empresa, cliente — score.rs/alert.rs/client.rs are stale)
│       └── services/ # Intentionally empty (mod.rs says so)
├── frontend/         # Dioxus WASM SPA
│   ├── src/main.rs   # Entrypoint: Login + Sidebar + MainArea (menu-based, no router)
│   ├── Dioxus.toml
│   └── tailwind.css  # Auto-detected by dx serve
└── docker-compose.yml
```

## Startup (order matters)

```bash
docker compose up -d                     # MongoDB on :27017
cd backend && MONGODB_URI=... cargo run  # Backend on :3000
cd frontend && dx serve --hot-reload     # Frontend on :8080
```

Backend reads `MONGODB_URI` from `.env` (or env var). Defaults to `mongodb://127.0.0.1:27017`.

## Gotchas

- **Root `main.rs` is stale.** Real entrypoints are `backend/src/main.rs` and `frontend/src/main.rs`.
- **`backend/src/services/mod.rs`** is intentionally empty — don't add code there unless the actual service files (`ocr_validation.rs`, `trust_score.rs`, `early_warning.rs`) are first populated.
- **`backend/src/models/score.rs`, `alert.rs`, `client.rs`** reference `chrono` which is NOT in `Cargo.toml`. These models are unused/uncompilable. Only `empresa.rs` and `cliente.rs` are wired in.
- **`Cargo.lock`** is gitignored (in `.gitignore` at root).
- **Frontend uses Dioxus 0.7** — no `cx`, `Scope`, `use_state`. Signals, `#[component]`, `rsx!`. See `frontend/AGENTS.md` for API reference.
- **Frontend app is not using the Dioxus Router** — it uses a simple `MenuState` enum with conditional rendering in the `App` component.
- **No tests, no CI, no lint/format checks** configured yet. Add them before production.

## API Endpoints (backend)

| Method | Path | Purpose |
|---|---|---|
| POST | `/api/login` | Empresa auth (correo + password) |
| GET | `/api/clientes/:curp` | Lookup client by CURP in PYMZA network |
| POST | `/api/update_status` | Update solicitud status |
| POST | `/api/ocr` | OCR validation (placeholder) |

## Secrets / Config

- `.env` file (gitignored) with `MONGODB_URI=""`
- Default MongoDB URI: `mongodb://127.0.0.1:27017`
- No auth tokens, no JWT — login returns static `"token-temporal-123"`


<!-- headroom:rtk-instructions -->
# RTK (Rust Token Killer) - Token-Optimized Commands

When running shell commands, **always prefix with `rtk`**. This reduces context
usage by 60-90% with zero behavior change. If rtk has no filter for a command,
it passes through unchanged — so it is always safe to use.

## Key Commands
```bash
# Git (59-80% savings)
rtk git status          rtk git diff            rtk git log

# Files & Search (60-75% savings)
rtk ls <path>           rtk read <file>         rtk grep <pattern>
rtk find <pattern>      rtk diff <file>

# Test (90-99% savings) — shows failures only
rtk pytest tests/       rtk cargo test          rtk test <cmd>

# Build & Lint (80-90% savings) — shows errors only
rtk tsc                 rtk lint                rtk cargo build
rtk prettier --check    rtk mypy                rtk ruff check

# Analysis (70-90% savings)
rtk err <cmd>           rtk log <file>          rtk json <file>
rtk summary <cmd>       rtk deps                rtk env

# GitHub (26-87% savings)
rtk gh pr view <n>      rtk gh run list         rtk gh issue list

# Infrastructure (85% savings)
rtk docker ps           rtk kubectl get         rtk docker logs <c>

# Package managers (70-90% savings)
rtk pip list            rtk pnpm install        rtk npm run <script>
```

## Rules
- In command chains, prefix each segment: `rtk git add . && rtk git commit -m "msg"`
- For debugging, use raw command without rtk prefix
- `rtk proxy <cmd>` runs command without filtering but tracks usage
<!-- /headroom:rtk-instructions -->
