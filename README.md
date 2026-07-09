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

- **Frontend**: SPA en WebAssembly con Sidebar + MainArea (tabs Open Banking, Servicios, INE/OCR) + Modal + Toast
- **Backend**: Dos endpoints REST (`POST /api/ocr`, `POST /api/update_status`) + pool de conexión a MongoDB

## Cómo Empezar

```bash
# Requisitos: Rust ≥ 1.80, Docker, wasm32-unknown-unknown, dioxus-cli

# 1. MongoDB
docker compose up -d

# 2. Backend (http://127.0.0.1:3000)
cd backend && cargo run

# 3. Frontend (http://localhost:8080)
cd frontend && dx serve --hot-reload
```

## Estructura

```
PYMZA/
├── backend/          # Axum server (Rust)
│   └── src/
│       ├── main.rs   # Rutas y entry point
│       ├── db.rs     # Conexión MongoDB
│       ├── models/   # Structs del dominio
│       └── services/ # Lógica de negocio
├── frontend/         # Dioxus WASM SPA
│   └── src/
│       ├── main.rs   # Componentes principales
│       ├── components/
│       └── views/
└── docker-compose.yml
```

## Licencias

| Tipo | Licencia |
|---|---|
| Código fuente | Apache 2.0 |
| Documentación y media | CC BY 4.0 |
