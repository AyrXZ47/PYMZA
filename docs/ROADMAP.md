# ROADMAP — PYMZA

Consolidación del checklist de visión de [`PYMZA.md`](../PYMZA.md) con el estado real del código.

> **Verificado contra el código** en `main` @ `f419c0f` (docs/roadmap, T-17). Cada estado se comprobó en `backend/src/main.rs` (10 rutas, `rg -c "\.route\("` = 10), `backend/src/models/` y `frontend/src/main.rs` (877 líneas), no copiado del checklist original.

## ✅ Lo ya hecho (verificado en código)

| Visión (PYMZA.md) | Estado real | Evidencia | Tarea en plan |
|---|---|---|---|
| Alta de empresas con contraseña | Hecho (backend). El frontend de registro sigue en curso | `POST /api/empresas` (valida correo/contraseña, evita duplicados) | T-07 (alta empresas), T-11 (hashing); registro UI: rama `feat/frontend-registro-token` sin merge |
| Hashing de contraseñas | Hecho | `hashear_password`/`password_correcta` con argon2id (PHC); seed con hash precomputado de `demo123`; tests `main.rs:554` | T-11 |
| Red de Alerta Temprana | Hecho | `POST /api/clientes/:curp/reportar` + campo `alerta` en `Cliente`; banner en panel de búsqueda (frontend) | T-08 (backend), T-09 (frontend) |
| Perfil de Cliente Global (reutilización) | Parcial: el perfil se reutiliza en la red, la privacidad por tienda no | `GET /api/clientes/:curp` devuelve el perfil existente en toda la red; `POST /api/clientes` rechaza duplicados por CURP | — (ver pendientes: multi-tenant) |
| Validación por OCR | Placeholder | `POST /api/ocr` devuelve JSON fijo | — (ver pendientes: KYC/OCR real) |
| Login con cuenta propia | Hecho (backend + logout frontend) | `POST /api/login` verifica hash argon2id; botón cerrar sesión; CORS restringido a orígenes locales | T-10, T-13 |
| Investigación buró / círculo de crédito | Hecho (investigación) | `PYMZA.md` [x]; rama `docs/investigacion-integraciones` | — |
| Investigación Open Banking (Belvo/Finerio) | Hecho (investigación) | `PYMZA.md` [x]; rama `docs/investigacion-integraciones` | — |
| System design | Hecho | `PYMZA.md` [x] | — |

## ⏳ Lo pendiente (checklist de visión → tarea del plan)

| Visión (PYMZA.md) | Estado actual | Tarea en plan |
|---|---|---|
| Frontend de registro / portal para empresas | Rama `feat/frontend-registro-token` en curso, sin merge en main | Por asignar |
| Base de datos multi-tenant (`empresa_id` en cada documento) | Sin implementar: las rutas por empresa reciben el id por path sin auth; no hay aislamiento entre empresas | Por asignar |
| Frontend para inversionistas | Sin implementar | Por asignar |
| Frontend de servicio técnico de PYMZA | Sin implementar | Por asignar |
| Login como portal espectacular que invite a contratar | Sin implementar (depende del registro) | Por asignar |
| Análisis y/o integración de FICO Score | Sin implementar (investigación previa hecha) | Por asignar |
| Integración con Stripe (o alternativa) | Sin implementar | Por asignar |
| Generación de contratos (PDF/recibo digital) | Sin implementar | Por asignar |
| KYC y OCR real (cámara/subida de archivos + procesado backend) | OCR es placeholder (`POST /api/ocr`, JSON fijo) | Por asignar |
| Privacidad del historial de compras entre tiendas | Sin implementar (el perfil global es único; la separación por empresa llegará con el multi-tenant) | Por asignar |
| Despliegue (Railway/docker) | Rama `infra/docker-fullstack` en curso, sin merge en main | Por asignar |

## 📋 Checklist original (PYMZA.md)

- [x] Investigar API de círculo de crédito o buró de crédito — hecha (docs/investigacion-integraciones)
- [x] Investigar plataformas de Open Banking (Belvo, Finerio) — hecha (docs/investigacion-integraciones)
- [x] System design — hecho
- [x] Validación por OCR — placeholder funcional (`POST /api/ocr`); OCR real pendiente (KYC)
- [x] Capacidad de dar de alta empresas con contraseña — backend hecho (T-07, T-11); UI de registro en curso
- [x] Red de Alerta Temprana — hecha (T-08, T-09)
- [ ] Frontend para inversionistas
- [ ] Frontend de servicio técnico de PYMZA
- [ ] Login como portal espectacular (invitar a contratar)
- [ ] Análisis y/o integración de FICO Score
- [ ] Perfil de Cliente Global con privacidad por tienda (multi-tenant)
- [ ] Integración con Stripe o alternativas
- [ ] Generación de contratos (PDF/recibo digital)
- [ ] KYC y OCR real
- [ ] Base de datos multi-tenant (`empresa_id`)
- [ ] Despliegue (Railway) — docker en curso (`infra/docker-fullstack`)

## 🎯 Alcance actual del producto (qué existe hoy)

SaaS B2B multi-tenant en ciernes: login/logout con hashing argon2id, alta de empresas, alta/búsqueda de clientes por CURP en la red (perfil único reutilizable, score base 550), evaluación/autorización de créditos con plan de pagos, cartera, dashboard y red de alerta temprana con banner. Frontend: Dioxus 0.7 en `frontend/src/main.rs` (~877 líneas, sin router). Backend: Axum con 10 rutas en `backend/src/main.rs`. Referencia completa: [`API.md`](API.md).
