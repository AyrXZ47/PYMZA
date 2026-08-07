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

- **Frontend**: SPA en WebAssembly con Sidebar + MainArea (Dashboard, Alta de Cliente, Cartera) + modal de 3 pasos para planes de pago. Todo el app vive en `frontend/src/main.rs`.
- **Backend**: 9 endpoints REST (login, alta/búsqueda de clientes, evaluación y autorización de créditos, cartera y dashboard) + pool de conexión a MongoDB.

## Cómo Empezar

Requisitos: Rust ≥ 1.80, `wasm32-unknown-unknown`, dioxus-cli (`dx`) y MongoDB.

```bash
# 1. MongoDB — opción A (Docker)
docker compose up -d
#    MongoDB — opción B (NixOS, mongod directo)
mongod --dbpath ~/.mongo-data --bind_ip 127.0.0.1 --port 27017

# 2. Seed demo (una vez por base nueva) — empresa: demo@pymza.mx / demo123
mongosh < backend/scripts/seed.js

# 3. Backend (http://127.0.0.1:3000)
cd backend && cargo run

# 4. Frontend (http://localhost:8080) — en NixOS primero regenera el CSS
cd frontend && ./tailwind.sh && dx serve
```

El backend lee `MONGODB_URI` de `backend/.env` (o variable de entorno). Para conectar a MongoDB Atlas, pega la connection string en `backend/.env`:

```bash
cd backend && echo 'MONGODB_URI="mongodb+srv://user:pass@cluster.mongodb.net/"' > .env
```

> **Nota NixOS**: `dx serve` sirve `assets/tailwind.css` tal cual (el `dx` de nixpkgs no compila Tailwind). Tras cambiar clases de Tailwind, regenera con `./tailwind.sh`.

## Estructura

```
PYMZA/
├── backend/          # Axum server (Rust)
│   └── src/
│       ├── main.rs   # Rutas y entry point (handlers inline)
│       ├── db.rs     # Conexión MongoDB (pool, lee MONGODB_URI)
│       ├── models/   # Structs del dominio (empresa, cliente, credito)
│       └── scripts/  # seed.js — datos demo
├── frontend/         # Dioxus WASM SPA
│   ├── src/main.rs   # Toda la UI: Login + Sidebar + MainArea
│   ├── tailwind.css  # Input de Tailwind
│   └── tailwind.sh   # Compila Tailwind a assets/tailwind.css
└── docker-compose.yml
```

## Licencias

| Tipo | Licencia |
|---|---|
| Código fuente | Apache 2.0 |
| Documentación y media | CC BY 4.0 |
