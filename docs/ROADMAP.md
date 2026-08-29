# ROADMAP — PYMZA

Consolidación del checklist de visión de [`PYMZA.md`](../PYMZA.md) con el estado real del código.

> **Verificado contra el código** en `main` tras la ola 1 (JWT real + multi-tenant + frontend modular; auditoría en `.workflow/audits/wave1.md`). Cada estado se comprobó en `backend/src/main.rs` (10 rutas, `rg -c "\.route\("` = 10), `backend/src/auth.rs` (JWT HS256), `backend/src/routes/` y `frontend/src/` (`main.rs` ~103 líneas + `api.rs` + `components/`), no copiado del checklist original.

## ✅ Lo ya hecho (verificado en código)

| Visión (PYMZA.md) | Estado real | Evidencia | Tarea en plan |
|---|---|---|---|
| Alta de empresas con contraseña | Hecho (backend); registro UI con auto-login en ola 2 | `POST /api/empresas` (valida correo/contraseña, evita duplicados); UI de registro propia + auto-login en curso (ola 2, executor-1) | T-07 (alta empresas), T-11 (hashing); ola 2 (registro UI) |
| Hashing de contraseñas | Hecho | `hashear_password`/`password_correcta` con argon2id (PHC); seed con hash precomputado de `demo123`; tests en `backend/src/auth.rs` | T-11 |
| Login con cuenta propia (JWT real) | Hecho | `POST /api/login` verifica el hash argon2id y emite JWT HS256 (claims `sub=correo`, `nombre`, exp 24h); el extractor `EmpresaSession` protege 8 rutas (401 con token ausente/inválido/expirado); CORS restringido a orígenes locales | T-10, T-13, ola 1 |
| Aislamiento multi-tenant | Hecho (tenant = correo de la empresa, sale del JWT) | `planes_pago` y `dashboard_stats` guardan `empresa = <correo del token>`; `GET /api/creditos` y `GET /api/dashboard` filtran por el token (la empresa ya no viaja en la URL); datos previos migrados con `backend/scripts/migrate_tenant.js` (idempotente) | Ola 1 |
| Red de Alerta Temprana | Hecho | `POST /api/clientes/:curp/reportar` + campo `alerta` en `Cliente`; banner en panel de búsqueda (frontend) | T-08 (backend), T-09 (frontend) |
| Perfil de Cliente Global (reutilización) | Parcial: el perfil se reutiliza en la red, la privacidad por tienda no | `GET /api/clientes/:curp` devuelve el perfil existente en toda la red; `POST /api/clientes` rechaza duplicados por CURP | — (ver pendientes: privacidad por tienda) |
| Validación por OCR | Placeholder | `POST /api/ocr` devuelve JSON fijo | — (ver pendientes: KYC/OCR real) |
| Frontend en módulos | Hecho | `frontend/src/main.rs` ~103 líneas (wiring) + `api.rs` (cliente HTTP, sesión) + `components/` (7 módulos); sin router (`MenuState` + render condicional) | Ola 1 |
| Investigación buró / círculo de crédito | Hecho (investigación) | `PYMZA.md` [x] | — |
| Investigación Open Banking (Belvo/Finerio) | Hecho (investigación) | `PYMZA.md` [x] | — |
| System design | Hecho | `PYMZA.md` [x] | — |

## ⏳ Lo pendiente (checklist de visión → tarea del plan)

| Visión (PYMZA.md) | Estado actual | Tarea en plan |
|---|---|---|
| Portal público (landing que venda, registro/login separados con CTA, tema claro/oscuro, `API_BASE` configurable) | En curso — Ola 2 (executor-1) | Ola 2 |
| Frontend para inversionistas | Sin implementar | Por asignar |
| Frontend de servicio técnico de PYMZA | Sin implementar | Por asignar |
| Análisis y/o integración de FICO Score | Sin implementar (investigación previa hecha) | Por asignar |
| Integración con Stripe (o alternativa) | Sin implementar | Por asignar |
| Generación de contratos (PDF/recibo digital) | Sin implementar | Por asignar |
| KYC y OCR real (cámara/subida de archivos + procesado backend) | OCR es placeholder (`POST /api/ocr`, JSON fijo) | Por asignar |
| Privacidad del historial de compras entre tiendas | Sin implementar (el perfil global es único por diseño de la red colaborativa) | Por asignar |
| Despliegue (Railway) | docker-compose listo; Railway pendiente | Ola 4 |

## 📋 Checklist original (PYMZA.md)

- [x] Investigar API de círculo de crédito o buró de crédito — hecha (docs/investigacion-integraciones)
- [x] Investigar plataformas de Open Banking (Belvo, Finerio) — hecha (docs/investigacion-integraciones)
- [x] System design — hecho
- [x] Validación por OCR — placeholder funcional (`POST /api/ocr`); OCR real pendiente (KYC)
- [x] Capacidad de dar de alta empresas con contraseña — backend (T-07, T-11) + registro UI (ola 2)
- [x] Red de Alerta Temprana — hecha (T-08, T-09)
- [x] Base de datos multi-tenant — hecha (ola 1: tenant = correo del JWT; `migrate_tenant.js` idempotente; ver `.workflow/audits/wave1.md`)
- [ ] Frontend para inversionistas
- [ ] Frontend de servicio técnico de PYMZA
- [ ] Portal público (landing que invite a contratar; registro/login con CTA) — en curso ola 2
- [ ] Análisis y/o integración de FICO Score
- [ ] Perfil de Cliente Global con privacidad por tienda
- [ ] Integración con Stripe o alternativas
- [ ] Generación de contratos (PDF/recibo digital)
- [ ] KYC y OCR real
- [ ] Despliegue (Railway) — docker-compose listo; Railway en ola 4

## 🎯 Alcance actual del producto (qué existe hoy)

SaaS B2B multi-tenant: registro y login con JWT real (HS256, exp 24h, `JWT_SECRET` obligatoria), tenant aislado por correo de la empresa (sale del token, nunca del path ni del body), alta/búsqueda de clientes por CURP en la red (perfil único reutilizable, score base 550), evaluación/autorización de créditos con plan de pagos, cartera, dashboard y red de alerta temprana con banner. Backend modular (Axum: `main.rs` = wiring + `routes/`, `models/`, `auth.rs`); frontend modular (Dioxus 0.7: `main.rs` + `api.rs` + `components/`). Referencia completa: [`API.md`](API.md). Auditoría ola 1: `.workflow/audits/wave1.md` (APPROVED WITH EXCEPTIONS: E1 docs — saldada en ola 2; E2 humo UI navegador, pendiente humano).

## 💭 Ideas diferidas (solo anotadas, no planificadas)

- **not-paid** (2026-08-27): fade-out del frontend si la factura vence (upstream `kleampa/not-paid`, fork local en `~/repos/not-paid`, ~100 líneas JS para `<head>`). `ponytail:` client-side y bypassable (JS off), y en Dioxus no hay `<head>` estático donde soltarlo; el enforcement real si algún día se necesita es un gate/licencia en el backend (Axum ya autentica). Idea mínima registrada; no tiene tarea en plan.
