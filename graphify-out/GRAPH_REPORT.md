# Graph Report - .  (2026-07-18)

## Corpus Check
- Corpus is ~11,647 words - fits in a single context window. You may not need a graph.

## Summary
- 127 nodes · 136 edges · 22 communities (17 shown, 5 thin omitted)
- Extraction: 93% EXTRACTED · 7% INFERRED · 0% AMBIGUOUS · INFERRED: 9 edges (avg confidence: 0.84)
- Token cost: 3,500 input · 2,100 output

## Community Hubs (Navigation)
- API Handlers & Routes
- Project Overview & Governance
- Frontend Components
- Dioxus Architecture
- App Shell & Navigation
- Backend Server & OCR
- Header SVG Branding
- Development Philosophy
- Database Connection
- Alert Model
- Score Model
- OpenCode Configuration
- Client Model
- Empresa Model
- Graphify Plugin
- Frontend Setup Docs
- Frontend Structure Docs

## God Nodes (most connected - your core abstractions)
1. `Client` - 8 edges
2. `buscar_cliente()` - 7 edges
3. `update_status()` - 6 edges
4. `login_empresa()` - 6 edges
5. `connect()` - 5 edges
6. `process_ocr()` - 5 edges
7. `Sidebar()` - 5 edges
8. `MainArea()` - 5 edges
9. `PYMZA Platform` - 5 edges
10. `Dioxus 0.7 Framework` - 5 edges

## Surprising Connections (you probably didn't know these)
- `Contributor Covenant` --references--> `PYMZA Platform`  [INFERRED]
  CODE_OF_CONDUCT.md → README.md
- `Contribution Guidelines` --references--> `PYMZA Platform`  [INFERRED]
  CONTRIBUTING.md → README.md
- `http_client()` --references--> `Client`  [EXTRACTED]
  frontend/src/main.rs → backend/src/models/client.rs
- `Credit Profiling` --references--> `PYMZA Platform`  [INFERRED]
  PYMZA.md → README.md
- `Trust Score` --references--> `MongoDB Database`  [INFERRED]
  PYMZA.md → README.md

## Import Cycles
- None detected.

## Hyperedges (group relationships)
- **Credit Profiling System** — pymza_credit_profiling, pymza_alternative_data_engine, pymza_trust_score, pymza_early_warning_network, pymza_collaborative_reputation, pymza_ocr_onboarding, pymza_client_profile_reuse, pymza_saas_b2b_multitenant [INFERRED 0.85]
- **PYMZA Technology Stack** — readme_dioxus_frontend, readme_axum_backend, readme_mongodb_database, readme_tailwind_css, readme_tokio_runtime, readme_docker_compose, docker_compose_mongodb_service [INFERRED 0.95]
- **Brand Animation System** — frontend_assets_header_brand_logo_animation, frontend_assets_header_cyan_swoosh_path, frontend_assets_header_red_swoosh_path, frontend_assets_header_animated_ui_elements, frontend_assets_header_complex_keyframe_animation [EXTRACTED 1.00]

## Communities (22 total, 5 thin omitted)

### Community 0 - "API Handlers & Routes"
Cohesion: 0.24
Nodes (14): buscar_cliente(), login_empresa(), LoginPayload, process_ocr(), Json, String, Value, update_status() (+6 more)

### Community 1 - "Project Overview & Governance"
Cohesion: 0.14
Nodes (14): Contributor Covenant, Contribution Guidelines, MongoDB Docker Service, Alternative Data Engine, Collaborative Reputation, Credit Profiling, Early Warning Network, OCR Identity Validation (+6 more)

### Community 2 - "Frontend Components"
Cohesion: 0.14
Nodes (8): Hero(), Element, Blog(), Element, Home(), Element, Navbar(), Element

### Community 3 - "Dioxus Architecture"
Cohesion: 0.17
Nodes (12): Dioxus Component Model, Dioxus 0.7 Framework, Dioxus Fullstack, Dioxus Router, Server Functions, Dioxus Signal API, Client Profile Reuse, SaaS B2B Multi-Tenant (+4 more)

### Community 4 - "App Shell & Navigation"
Cohesion: 0.40
Nodes (8): App(), Login(), MainArea(), MenuState, Element, String, Sidebar(), Signal

### Community 5 - "Backend Server & OCR"
Cohesion: 0.24
Nodes (7): process_ocr(), Json, String, Value, update_status(), UpdateStatusPayload, UpdateStatusPayload

### Community 6 - "Header SVG Branding"
Cohesion: 0.33
Nodes (7): Animated UI Elements, Brand Logo Animation, Color Palette (Cyan Red Orange Dark Gray), Complex Keyframe Animation System, Cyan Swoosh Path, Header SVG, Red Swoosh Path

### Community 7 - "Development Philosophy"
Cohesion: 0.40
Nodes (5): Graphify, Ponytail Lazy Senior Dev, RTK Rust Token Killer, Shortest Working Diff, YAGNI

### Community 8 - "Database Connection"
Cohesion: 0.40
Nodes (4): connect(), Box, Error, Result

### Community 9 - "Alert Model"
Cohesion: 0.40
Nodes (4): Alert, DateTime, String, Utc

### Community 10 - "Score Model"
Cohesion: 0.40
Nodes (4): DateTime, String, Utc, Score

### Community 11 - "OpenCode Configuration"
Cohesion: 0.50
Nodes (3): plugin, $schema, .opencode/plugins/graphify.js

## Knowledge Gaps
- **24 isolated node(s):** `$schema`, `.opencode/plugins/graphify.js`, `Collaborative Reputation`, `Client Profile Reuse`, `OCR Identity Validation` (+19 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **5 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Client` connect `API Handlers & Routes` to `Database Connection`?**
  _High betweenness centrality (0.041) - this node is a cross-community bridge._
- **Why does `http_client()` connect `API Handlers & Routes` to `App Shell & Navigation`?**
  _High betweenness centrality (0.027) - this node is a cross-community bridge._
- **Why does `Axum Backend` connect `Dioxus Architecture` to `Project Overview & Governance`?**
  _High betweenness centrality (0.023) - this node is a cross-community bridge._
- **What connects `$schema`, `.opencode/plugins/graphify.js`, `Collaborative Reputation` to the rest of the system?**
  _24 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Project Overview & Governance` be split into smaller, more focused modules?**
  _Cohesion score 0.14285714285714285 - nodes in this community are weakly interconnected._
- **Should `Frontend Components` be split into smaller, more focused modules?**
  _Cohesion score 0.14285714285714285 - nodes in this community are weakly interconnected._