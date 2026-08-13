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
| Infra | Docker Compose (`mongo:latest` + named volume) |

## Structure

```
PYMZA/
├── backend/          # Axum server
│   └── src/
│       ├── main.rs   # ENTRYPOINT: all 10 routes live here (handlers are inline)
│       ├── db.rs     # MongoDB pool (max 10); hardcodes 127.0.0.1 to avoid IPv6 timeout
│       └── models/   # Domain structs; mod.rs wires in empresa, cliente, credito
├── frontend/         # Dioxus WASM SPA — entire app is src/main.rs (~877 lines), no router
│   ├── src/
│   │   └── main.rs   # Entrypoint: Login + Sidebar + MainArea (MenuState enum, conditional render)
│   ├── tailwind.css  # Tailwind input (tracked, 1 line). `dx serve` auto-compiles → assets/tailwind.css
│   └── clippy.toml   # Only lint config in the repo (Dioxus signal read-locks over await)
└── docker-compose.yml
```

## Startup (order matters)

```bash
# MongoDB on :27017 (Docker)
docker compose up -d
# o en NixOS sin docker (mongodb del módulo rust-dev de nixos-config):
mongod --dbpath ~/.mongo-data --bind_ip 127.0.0.1 --port 27017

# Seed demo (una vez por base nueva) — empresa: demo@pymza.mx / demo123
mongosh < backend/scripts/seed.js

cd backend && MONGODB_URI=... cargo run  # Backend on 127.0.0.1:3000
cd frontend && dx serve                  # Frontend on :8080 (NO uses --hot-reload: en dx 0.7.9 pide un valor: --hot-reload true)
```

Backend reads `MONGODB_URI` from `.env` (or env var). Defaults to `mongodb://127.0.0.1:27017`. Frontend hardcodes `http://127.0.0.1:3000` for all API calls (main.rs) — change it in one place.

On NixOS (see `modules/apps/rust-dev.nix` in yovick/nixos-config): after the first `nixos-rebuild switch`, run once per machine `rustup default stable && rustup target add wasm32-unknown-unknown`. MongoDB's license is SSPL (unfree).

## Tailwind (NixOS gotcha)

**`dx` from nixpkgs does NOT auto-compile Tailwind** — it only copies `assets/tailwind.css` (must already exist, or `asset!` fails to hash and the app loads unstyled). The compiled CSS is **committed** (unignored in `.gitignore`). After adding/changing Tailwind classes, regenerate:

```bash
cd frontend && ./tailwind.sh        # o: ./tailwind.sh --watch durante desarrollo
```

## Gotchas

- **Building the backend needs OpenSSL dev libs** on Linux (mongodb driver ships with `openssl-tls` feature).
- **`Cargo.lock` is gitignored** (root `.gitignore`).
- **Frontend uses Dioxus 0.7** — no `cx`, `Scope`, `use_state`. Signals, `#[component]`, `rsx!`, `spawn`. `frontend/AGENTS.md` is the auto-loaded Dioxus 0.7 API reference — follow it.
- **`dioxus` is pinned to `=0.7.9`** in `frontend/Cargo.toml` to match the `dx` CLI that ships in nixpkgs (0.7.9). Bumping one without the other triggers `dx` version-mismatch warnings.
- **No router:** app uses a `MenuState` enum + conditional rendering. The `router` feature is enabled in `frontend/Cargo.toml` but unused.
- **`frontend/tailwind.css`** (tracked) is the Tailwind input; the compiled `assets/tailwind.css` is gitignored/generated.
- **No tests, no CI.** Only lint config is `frontend/clippy.toml`. `cargo test`/`dx check` are the only verification available.
- **Login** — password hashed with argon2id (PHC) in `empresas`; token is still static `"token-temporal-123"` (no JWT, routes don't validate it).

## API Endpoints (backend)

Collections: `empresas`, `clientes`, `planes_pago`, `dashboard_stats`.

| Method | Path | Purpose |
|---|---|---|
| POST | `/api/login` | Empresa auth (correo + password) |
| POST | `/api/empresas` | Alta de empresa nueva (valida correo y contraseña, evita duplicados) |
| GET | `/api/clientes/:curp` | Lookup client by CURP in PYMZA network |
| POST | `/api/clientes` | Alta de cliente nuevo (valida CURP, evita duplicados; score base 550) |
| POST | `/api/clientes/:curp/reportar` | Alerta temprana: marca al cliente como moroso/desaparecido con motivo (red colaborativa) |
| POST | `/api/ocr` | OCR validation (placeholder, fixed JSON) |
| POST | `/api/creditos/evaluar` | Evaluate credit: rate by plazo (3m=3% … 12m=15%), approve/reject by score, build payment plan |
| POST | `/api/creditos/autorizar` | Insert `planes_pago` + upsert `dashboard_stats` |
| GET | `/api/creditos/:empresa` | Active `planes_pago` for a company (Cartera) |
| GET | `/api/dashboard/:empresa` | Dashboard stats per empresa |

## Secrets / Config

- `backend/.env` (gitignored) with `MONGODB_URI=""` — see `.env.example`. dotenvy busca `.env` desde el cwd hacia arriba, así que sirve desde `backend/` o la raíz.
- Never print or commit `MONGODB_URI`; to connect a real Atlas DB the user writes the URI into `backend/.env` themselves — verify connectivity via the backend log (`Pool de conexiones MongoDB inicializado`), never via echoing the secret.
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

---

# AI Workflow: planner → executors → auditor

This repo runs a multi-instance wave workflow. The plan and every handoff live in committed files under `.workflow/` — never only in a chat context. Sessions are disposable; the files are the memory.

## Roles

- **Planner** (fresh session, strongest available model): reads the project idea and this repo, writes `.workflow/plan.md` with the wave list and the file-ownership map, and writes one brief per executor under `.workflow/briefs/`. Details ONLY the next wave (rolling plan).
- **Executor** (one per brief, cheaper model): `git worktree add` its own branch, reads its brief, implements, runs the brief's verify command, commits. Touches only the files it owns.
- **Merger** (medium model): merges the wave branches into `main` in the order of the plan's integration plan, runs build + tests on the integrated tree, pushes `main`. On conflict: STOPS and reports — never resolves conflicts with its own criteria.
- **Auditor** (fresh session — never the planner's session — strongest model): reviews the INTEGRATED tree (merged worktrees) against `.workflow/audit-checklist.md`. Evidence over narration: every check is a command it runs; a claim without output is a failed check.

## Wave rules

1. A wave = parallel executors with disjoint file ownership. Two executors never own the same file in the same wave; if they need it, sequence them.
2. Every wave ends with: integration (the merger merges the wave branches into main, then build + tests) → audit. The next wave starts only after the audit passes or records explicit exceptions in `.workflow/plan.md`.
3. Rolling plan: only the next wave is detailed. After each audit the planner re-plans the next wave from the decision log.
4. Release gate: anything that will be distributed runs `skills/security-audit` first. Zero CRITICAL/HIGH findings, or documented exceptions. Never skip it.
5. Lazy rules apply to everyone, including the auditor: the best audit is the smallest audit that catches the real failure.

## Commit rules (mandatory)

- Every commit uses conventional commits: `feat:`, `fix:`, `chore:`, `docs:`, `refactor:`, `test:`, `perf:`, `style:`, `build:`, `ci:`, `revert:`. Scope optional: `feat(core):`.
- Short summary: imperative, lowercase, one line, under ~72 chars. No AI attribution, no trailers, no prose.
- One logical change per commit. `feat:`/`fix:` change behavior; `chore:` doesn't.
- Executors commit ONLY their owned files, one commit per task.
- **Branch isolation (mandatory):** every executor commits AND pushes ONLY to its own worktree branch. Never push to `main` or to another executor's branch; never merge, rebase, or fast-forward anyone else's branch. `git push origin <your-branch>` after each commit, so the work survives the session without touching parallel instances.
- Committing is not a reward: if the diff can't be described in one short line, split it.

## Skills in this repo

- `skills/security-audit` — Cloudflare 6-phase security audit (recon → parallel hunt → adversarial validation → report → structured output → independent verification). Trigger: "security audit", "find vulnerabilities", "pen-test". Required at release gates.
- `skills/ponytail-review` — diff review that hunts over-engineering. Trigger: "review for over-engineering", "what can we delete".
- `skills/ponytail-audit` — repo-wide over-engineering scan. Trigger: "audit this codebase", "find bloat".
- `skills/ponytail-debt` — harvests every `ponytail:` comment into a debt ledger. Trigger: "ponytail debt", "list the shortcuts".
- `skills/ponytail-gain` / `skills/ponytail-help` — ponytail impact scoreboard and reference card.
