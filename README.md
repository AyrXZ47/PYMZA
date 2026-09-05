# PYMZA — Perfilación de Crédito y Cobranza para PYMES

Plataforma SaaS B2B multi-tenant que perfila riesgo crediticio usando fuentes de datos alternativas (pagos de servicios, historial en red) y una red colaborativa de alerta temprana entre negocios. Construida íntegramente en **Rust**.

## Stack

| Capa | Tecnología |
|---|---|
| **Frontend** | Dioxus 0.7 (Rust → WASM) + Tailwind CSS |
| **Backend** | Axum / Tokio |
| **Base de Datos** | MongoDB 7+ |
| **Infraestructura** | Docker Compose |

## Arquitectura

```
Frontend (Dioxus WASM) ──HTTP/JSON──> Backend (Axum/Tokio) ──> MongoDB
```

- **Frontend**: SPA en WebAssembly, modular: `main.rs` (wiring, ~103 líneas) + `api.rs` (cliente HTTP compartido, `API_BASE`, sesión persistente en `localStorage`) + `components/` (login, alta de cliente, modal de plan de pagos, cartera, dashboard, sidebar). Sin router: `MenuState` + rendering condicional.
- **Backend**: 10 endpoints REST (login, alta de empresas, alta/búsqueda de clientes, reporte de morosidad, evaluación y autorización de créditos, cartera y dashboard) + pool de conexión a MongoDB. Modular: `main.rs` (wiring) + `routes/`, `models/` y `auth.rs` (JWT HS256 + argon2id). El tenant (empresa) sale del JWT, nunca del path ni del body.

## Documentación

- [`docs/API.md`](docs/API.md) — referencia completa de los 10 endpoints (payloads y respuestas).
- [`docs/ROADMAP.md`](docs/ROADMAP.md) — checklist de visión de PYMZA.md consolidado con el estado real del código.

## Cómo Empezar

Requisitos: Rust ≥ 1.80, `wasm32-unknown-unknown`, dioxus-cli (`dx`) y MongoDB.

```bash
# 1. MongoDB — opción A (Docker)
docker compose up -d
#    MongoDB — opción B (NixOS, mongod directo)
mongod --dbpath ~/.mongo-data --bind_ip 127.0.0.1 --port 27017

# 2. Seed demo (una vez por base nueva) — empresa: demo@pymza.mx / demo1234
mongosh < backend/scripts/seed.js

# 3. Backend (http://127.0.0.1:3000)
cd backend && cargo run

# 4. Frontend (http://localhost:8080) — en NixOS primero regenera el CSS
cd frontend && ./tailwind.sh && dx serve
```

El backend lee `MONGODB_URI` y `JWT_SECRET` de `backend/.env` (o variables de entorno). `JWT_SECRET` es obligatoria (el backend falla al arrancar con mensaje claro si falta): firma los JWTs de sesión (HS256, exp 24h). Para conectar a MongoDB Atlas, pega la connection string en `backend/.env`:

```bash
cd backend && printf 'MONGODB_URI="mongodb+srv://user:pass@cluster.mongodb.net/"\nJWT_SECRET="cambia-este-secreto"\n' > .env
```

> **Autenticación**: salvo `POST /api/login` y `POST /api/empresas`, todas las rutas requieren el header `Authorization: Bearer <jwt>`; sin token válido devuelven 401.

> **Nota NixOS**: `dx serve` sirve `assets/tailwind.css` tal cual (el `dx` de nixpkgs no compila Tailwind). Tras cambiar clases de Tailwind, regenera con `./tailwind.sh`.

## Estructura

```
PYMZA/
├── backend/          # Axum server (Rust)
│   ├── src/
│   │   ├── main.rs   # Entry point: wiring del Router (handlers en routes/)
│   │   ├── auth.rs   # JWT HS256 (jsonwebtoken v9), extractor EmpresaSession, argon2id, CORS
│   │   ├── db.rs     # Conexión MongoDB (pool, lee MONGODB_URI)
│   │   ├── models/   # Structs del dominio (empresa, cliente, credito)
│   │   └── routes/   # Handlers por dominio (empresa, cliente, credito)
│   └── scripts/      # seed.js — datos demo
├── frontend/         # Dioxus WASM SPA
│   ├── src/
│   │   ├── main.rs   # Wiring: App + MenuState + rendering condicional
│   │   ├── api.rs    # Cliente HTTP, API_BASE y sesión (localStorage)
│   │   └── components/  # login, alta_cliente, plan_modal, cartera, dashboard, sidebar
│   ├── tailwind.css  # Input de Tailwind
│   └── tailwind.sh   # Compila Tailwind a assets/tailwind.css
└── docker-compose.yml
```

## Licencias

| Tipo | Licencia |
|---|---|
| Código fuente | Apache 2.0 |
| Documentación y media | CC BY 4.0 |
