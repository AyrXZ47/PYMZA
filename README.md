# 🚀 PYMZA — Perfilación de Crédito y Cobranza para PYMES

> **App para perfilación de crédito y cobranza dirigido a empresas para PYMES.**  
> Motor de datos alternativos + red de alerta temprana + validación por OCR.  
> SaaS B2B Multi-Tenant construido íntegramente en **Rust**.

---

## 📋 Tabla de Contenido

- [El Concepto](#-el-concepto)
- [Stack Tecnológico](#️-stack-tecnológico)
- [Arquitectura](#-arquitectura)
- [Roadmap](#-roadmap)
- [Cómo Empezar](#-cómo-empezar)
  - [Prerrequisitos](#prerrequisitos)
  - [Backend](#backend)
  - [Frontend](#frontend)
- [API Endpoints](#-api-endpoints)
- [Estructura del Proyecto](#-estructura-del-proyecto)
- [Licencias](#-licencias)

---

## 🎯 El Concepto

### Elevator Pitch
Plataforma SaaS B2B multi-tenant que permite a pequeñas y medianas empresas ofrecer productos a crédito de manera estructurada y fiable. El sistema perfila el riesgo crediticio de los solicitantes usando **fuentes de datos alternativas** (pagos de servicios básicos, historial en la red) y una **red colaborativa de alerta temprana** entre negocios.

### Pilares del Sistema

| Pilar | Descripción |
|---|---|
| **🧠 Motor de Datos Alternativos** | Perfila riesgo analizando historial de pagos de servicios (CFE, agua, telecomunicaciones). Asigna un "Score de Confianza" a clientes sin historial crediticio bancario. |
| **🛡️ Red de Alerta Temprana** | Sistema de reputación colaborativo entre PYMES. Si un cliente comete fraude o desaparece con una deuda, la red actualiza su perfil y alerta a otros negocios. |
| **📄 Validación por OCR** | Onboarding con verificación de identidad vía OCR para prevenir suplantación y fraudes. |
| **🏢 Multi-Tenant** | Cada empresa ve solo sus propios clientes, métricas y planes de pago. Un perfil de cliente se reutiliza entre empresas para evitar duplicados. |

### Flujo de Alto Nivel

```
Empresa inicia sesión
       │
       ▼
Panel → Dar de alta nuevo cliente
       │
       ▼
Captura de datos + INE (OCR) + documentos
       │
       ▼
Backend evalúa Score de Confianza
       │
       ▼
Recomendación → Aprobar / Rechazar crédito
       │
       ▼
Panel de Plan de Pagos (plazos, intereses, CAT)
```

---

## 🛠️ Stack Tecnológico

| Capa | Tecnología | Versión |
|---|---|---|
| **Frontend** | [Dioxus](https://dioxuslabs.com) 0.7 (Rust WASM) + Tailwind CSS | `0.7.1` |
| **Backend** | [Axum](https://github.com/tokio-rs/axum) 0.6 sobre Tokio | `0.6` |
| **Base de Datos** | MongoDB 7+ (vía driver `mongodb` 2.4) | `2.4.0` |
| **Infraestructura** | Docker Compose (MongoDB local) | `3.8` |

### Dependencias Principales

**Backend** (`backend/Cargo.toml`):
- `axum` + `tower-http` (CORS) — servidor HTTP
- `mongodb` 2.4 — driver nativo de MongoDB
- `tokio` — runtime asíncrono
- `serde` / `serde_json` — serialización

**Frontend** (`frontend/Cargo.toml`):
- `dioxus` 0.7 con router — UI reactiva compilada a WASM
- `reqwest` 0.12 — cliente HTTP
- `gloo-timers` 0.3 — temporizadores para auto-dismiss de notificaciones
- `serde_json` — parseo de respuestas JSON

---

## 🏗️ Arquitectura

### Diagrama de Componentes

```
┌──────────────┐     HTTP/JSON     ┌─────────────────┐     MongoDB Wire     ┌──────────┐
│   Frontend   │ ────────────────> │    Backend      │ ──────────────────> │ MongoDB  │
│  (Dioxus     │                   │  (Axum/Tokio)   │                     │ (Docker) │
│   WASM)      │ <──────────────── │                 │ <────────────────── │          │
└──────────────┘                   └─────────────────┘                     └──────────┘
       │                                  │
       │                                  ├─ /api/ocr              → process_ocr()
       │                                  ├─ /api/update_status    → update_status()
       │                                  └─ db::connect()        → Pool MongoDB
       │
  ┌────┴─────┐
  │ Navegador│
  │ Web      │
  └──────────┘
```

### Frontend (Dioxus WASM)

El frontend es una **Single Page Application** compilada a WebAssembly. Los componentes principales:

- **`App`** — Layout raíz con `Sidebar` + `MainArea`
- **`Sidebar`** — Navegación lateral (Dashboard, Evaluación, Clientes, Red PYME)
- **`MainArea`** — Panel principal con tabs (Open Banking, Servicios, INE/OCR) + Action Bar (Aprobar/Rechazar) + Modal + Toast
- **Modal** — Overlay emergente para aprobación/rechazo con animaciones:
  - `Loading` → spinner giratorio
  - `Success` → círculo verde + palomita (pop-in animation)
  - `Error` → círculo rojo + equis (pop-in animation)
- **Toast** — Notificación auto-dismissable (12s) en esquina superior derecha

### Backend (Axum + MongoDB)

Servidor HTTP asíncrono con dos endpoints REST:

- `POST /api/ocr` — Simula escaneo OCR y retorna `id` + `extracted_name`
- `POST /api/update_status` — Actualiza estado de solicitud (`Aprobado`/`Rechazado`) en MongoDB

La conexión a MongoDB se configura vía variable de entorno `MONGODB_URI` (por defecto `mongodb://127.0.0.1:27017`).

---

## 🗺️ Roadmap

- [x] Investigación API Buró de Crédito / Círculo de Crédito
- [x] Investigación plataformas Open Banking (Belvo, Finerio)
- [x] System Design inicial
- [x] Validación por OCR (prototipo)
- [ ] MVP funcional multi-tenant
- [ ] Panel de planes de pago
- [ ] Red de alerta temprana colaborativa
- [ ] Portal para inversionistas (métricas)
- [ ] Portal de soporte técnico PYMZA
- [ ] Desktop app con Tauri

---

## 🚀 Cómo Empezar

### Prerrequisitos

| Herramienta | Versión |
|---|---|
| [Rust](https://rustup.rs) | ≥ 1.80 |
| [Docker](https://docker.com) + Docker Compose | Cualquier reciente |
| `wasm32-unknown-unknown` target | `rustup target add wasm32-unknown-unknown` |
| [Dioxus CLI](https://dioxuslabs.com/learn/0.7/CLI/installation) | `cargo install dioxus-cli` |

### Backend

```bash
# 1. Levantar MongoDB
docker compose up -d

# 2. Configurar variable de entorno (opcional)
export MONGODB_URI="mongodb://127.0.0.1:27017"

# 3. Iniciar servidor
cd backend
cargo run
```

El servidor arranca en `http://127.0.0.1:3000`.

### Frontend

```bash
cd frontend
dx serve --hot-reload
```

La aplicación se abre en `http://localhost:8080` con recarga en caliente.

### Notas

- El frontend se conecta al backend en `http://127.0.0.1:3000`.
- Si MongoDB no está disponible, el backend falla al arrancar.
- El endpoint `/api/ocr` actualmente devuelve datos simulados (hardcoded).

---

## 📡 API Endpoints

### `POST /api/ocr`

Simula un escaneo OCR de identificación.

**Respuesta (200):**
```json
{
  "status": "success",
  "id": "12345",
  "extracted_name": "Janeth Ramos Zamora"
}
```

### `POST /api/update_status`

Actualiza el estado de una solicitud en MongoDB.

**Body:**
```json
{
  "id": "12345",
  "estado": "Aprobado"
}
```

**Respuesta (200):**
```json
{
  "status": "success"
}
```

---

## 📂 Estructura del Proyecto

```
PYMZA/
├── backend/                        # Servidor Axum (Rust)
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs                 # Entry point + rutas Axum
│       ├── db.rs                   # Conexión a MongoDB (pool)
│       ├── models/                 # Structs del dominio
│       │   ├── mod.rs
│       │   ├── client.rs
│       │   ├── score.rs
│       │   └── alert.rs
│       └── services/               # Lógica de negocio
│           ├── mod.rs
│           ├── ocr_validation.rs
│           ├── trust_score.rs
│           └── early_warning.rs
├── frontend/                       # SPA Dioxus WASM
│   ├── Cargo.toml
│   ├── Dioxus.toml                 # Configuración de Dioxus
│   └── src/
│       ├── main.rs                 # Entry point + todos los componentes
│       ├── components/
│       │   ├── mod.rs
│       │   └── hero.rs
│       └── views/
│           ├── mod.rs
│           ├── home.rs
│           ├── blog.rs
│           └── navbar.rs
├── docker-compose.yml              # MongoDB container
├── .env.example                    # Variables de entorno
├── PYMZA.md                        # Documentación del concepto
└── README.md                       # Este archivo
```

---

## 📜 Licencias

Este repositorio usa un modelo multi-licencia:

| Tipo | Licencia | Archivo |
|---|---|---|
| **Código fuente** | Apache 2.0 | [`LICENSE-SOFTWARE`](LICENSE-SOFTWARE) |
| **Hardware** | CERN-OHL-P v2 | [`LICENSE-HARDWARE`](LICENSE-HARDWARE) |
| **Documentación y media** | CC BY 4.0 | [`LICENSE-MEDIA`](LICENSE-MEDIA) |

---

> 🧩 Proyecto activo — Junio 2026  
> 💡 Idea original: Yovick RZ — [@AyrXZ47](https://github.com/AyrXZ47)
