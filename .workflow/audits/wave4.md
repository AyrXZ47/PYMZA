# Auditoría — Ola 4: cartera viva (pagos, gráficas, favicon)

**Fecha:** 2026-09-04 · **Auditor:** sesión fresca (no planner) · **Árbol:** main @ `041a74e` (integrada por el integrador, build+tests verdes)
**Veredicto: APPROVED** — la ola 5 puede arrancar. Sin bloqueos; 3 notas informativas (N1–N3).

## 1. Integridad de integración

- **Ambas ramas fusionadas en main:** `262c428 merge: wave 4 wave4-executor-1`, `6f7bc96 merge: wave 4 wave4-executor-2`. Evidencia: `git log --oneline -15`.
- **git status limpio:** `nothing to commit, working tree clean`; `git stash list` vacío. Branch `main` up to date con `origin/main`.
- **Mapa de propiedad respetado** (`git log --stat` por rama, base `5490842`):
  - executor-1 (`3666cf5`, `ea4eb82`, `528f75d`): solo `backend/src/**`, `docs/API.md`, `.env.example`. No tocó `backend/Cargo.toml` (correcto: cero deps).
  - executor-2 (`ea78930`, `803b7fe`, `09e9f33`, `645796c`, `b2c4251`): solo `frontend/src/**`, `frontend/Dioxus.toml`, `frontend/assets/**`.
  - `frontend/Dioxus.toml`: diff `5490842..6f7bc96` = **solo** `title = "PYMZA — Crédito con cobranza respaldada"` (única key permitida). ✔

## 2. Build y tests (árbol integrado)

| Comando | Resultado |
|---|---|
| `cd backend && cargo build` | `Finished dev profile` ✔ |
| `cd backend && cargo test` | **46 passed; 0 failed** (23 previos + 23 nuevos: estados, vencimientos, buckets, shape del resumen, plantilla) |
| `cd frontend && cargo check --target wasm32-unknown-unknown` | `Finished dev profile` ✔ |
| `cd frontend && cargo test` | **22 passed; 0 failed** (parseo resumen, semáforo morosidad, siguiente cuota impaga, builder POST pagos) |
| `./tailwind.sh` + `git diff --stat assets/tailwind.css` | **diff vacío** — CSS commiteado sin drift ✔ |

## 3. Humo e2e en vivo (backend integrado vs Atlas, `BIND_ADDR=127.0.0.1:3010`)

> El backend en :3000 era un build viejo (sin campos nuevos); se corrió el binario integrado en :3010 contra la misma DB. Detenido tras la auditoría.

**Registro de pago — escalera de validación:**

| Caso | HTTP | Evidencia (respuesta) |
|---|---|---|
| Pago válido cuota 1, monto 28.75 (plan `6a77f714…`, taladro) | success | `estado: Activo, pagadas: 1` |
| Duplicado cuota 1 | **400** | `"Cuota ya registrada"` |
| Cuota 99 (> plazo 12) | **400** | `"Cuota fuera de rango: debe estar entre 1 y 12"` |
| Monto 99.99 ≠ pago_mensual | **400** | `"El monto debe ser igual al pago mensual del plan ($28.75)"` |
| Plan ajeno (otro tenant) | **404** | `"Plan no encontrado"` — sin fuga de datos |
| Token inventado | **401** | extractor JWT |

**Ciclo de vida del plan (en vivo):** cuotas 2→12 pagadas en secuencia → `pagadas: n` incremental; **cuota 12 → `estado: Liquidado`**, persistido (`GET /api/creditos` lo devuelve con `cuotas_pagadas: 12, cuotas_vencidas: 0`).
`Moroso` en vivo no reproducible hoy: ningún plan tiene cuota vencida (vencimientos desde 2026-09-08, hoy 09-04). Cubierto por tests puros `estado_plan_cuota_atrasada_es_moroso`, `buckets_aging_cubren_los_limites`. ✔

**`GET /api/creditos`:** expone `_id` (hex), `cuotas_pagadas`, `cuotas_vencidas` por plan. ✔

**`GET /api/creditos/resumen` — shape exacta al contrato:**
```
top keys: aging, cobrado_vs_por_cobrar, distribucion_montos, flujo_proyectado, tasa_morosidad, top_deudores
cobrado_vs_por_cobrar: 6 meses 2026-04..2026-09; último: {mes: 2026-09, cobrado: 28.75, por_cobrar: 380.91}
  (por_cobrar 380.91 = 239.58+141.33; la cuota de sept del taladro ya pagada no cuenta — lógica correcta)
tasa_morosidad: 0.0 (f64)
flujo_proyectado: [{30,380.91},{60,790.57},{90,1200.23}] → tras liquidar: [{30,380.91},{60,761.82},{90,1142.73}] ✔ resta cuotas del plan liquidado
aging: 4 buckets (0-30/31-60/61-90/90+), todos 0.0 (nada vencido — consistente)
top_deudores: [{cliente_curp, nombre: "Pepito" (join con clientes), saldo: 2874.96}]; plan Liquidado excluido
distribucion_montos: 3 buckets 0-1k/1k-5k/5k+
```

**Tenant scoping (en vivo, 2 tenants):** tenant nuevo `auditor4@pymza.mx` → `resumen` vacío (tasa 0, `top_deudores: []`, sin cobrados) aunque `nueva@empresa.mx` tenía 3 planes + pagos. Sin cruce de datos. ✔

## 4. Plantilla WhatsApp + TTL

- **Builder puro + test:** `otp::tests::payload_plantilla_lleva_el_codigo_como_parametro_del_body` pasa. Defaults `WHATSAPP_TEMPLATE`=`pymza_otp_verification`, `WHATSAPP_TEMPLATE_LANG`=`es`. `WHATSAPP_WABA_ID` no introducido (según brief). ✔
- **Fallo suave:** `otp.rs:118-119` — HTTP error y error de red → `eprintln!` y continuar, sin panic. ✔
- **TTL index (O2 de ola 3 — RESUELTO):** `mongosh` contra Atlas:
  `[{"name":"expira_en_1","keys":{"expira_en":1},"ttl":0}]` ✔
- `.env.example`: `WHATSAPP_TEMPLATE=""` y `WHATSAPP_TEMPLATE_LANG=""` presentes. `docs/API.md` documenta `/pagos`, `/resumen`, plantilla y TTL. ✔

## 5. Favicon / título

- `frontend/assets/favicon.svg` existe (294 B, <1 KB), `main.rs:21` `asset!("/assets/favicon.svg")`, `main.rs:98` `document::Link { rel: "icon" }`. `Dioxus.toml` title ≠ "frontend". ✔
- Humo visual de la pestaña/gráficas en navegador queda para V (el plan lo asigna al humano); backend + shape verificados aquí.

## 6. Disciplina ponytail

- **Cero deps nuevas:** `git diff 5490842..041a74e -- '**/Cargo.toml'` vacío. Gráficas = SVG puro (332 líneas en `charts.rs`, 4 primitivas). ✔
- **`ponytail:` comments:** charts.rs ×1 (sin interactividad → techo librería), credito.rs ×4, otp.rs ×2. ✔
- Smallest diff razonable: resumen calculado en memoria sin framework de agregación (volumen de PYME), como pide el brief. Sin abstracciones no pedidas.

## 7. Seguridad

- **Sin secretos commiteados:** scan del diff de la ola (patrones `mongodb+srv://`, `api_key`, tokens largos) → solo prosa del decision log; `.env` gitignored. ✔
- **Trust boundaries:** `/pagos` valida propiedad del plan (tenant del token), rango de cuota, duplicado y monto — probado en vivo arriba. ✔
- No es release gate (ola 6): `skills/security-audit` queda pendiente para esa ola.

## Notas informativas

- **N1 (owner V):** login `demo@pymza.mx` devuelve 401 en Atlas (credenciales demo ausentes/diferentes) — misma raíz que O1/O3 de la ola 3 (re-seed de datos demo). No bloquea.
- **N2:** estado `Moroso` verificado solo por tests unitarios hoy (ningún plan vencido aún; primero vence 2026-09-08). Verificar en vivo cuando ocurra el primer vencimiento real.
- **N3 (owner V):** huella de humo en Atlas tras esta auditoría: plan `6a77f714…` (taladro, `nueva@empresa.mx`) quedó `Liquidado` con 12 pagos (procedimiento del propio plan de integración); tenant de prueba `auditor4@pymza.mx` eliminado. Se suma a O3 (limpieza de datos de humo).
- **V debe reiniciar su backend de dev en :3000** — sigue sirviendo el binario viejo sin los campos de la ola 4.

## Veredicto

**APPROVED.** Todo lo planeado para la ola 4 está implementado, verificado con evidencia de comandos y dentro del mapa de propiedad. Ola 5 (KYC/OCR + score alternativo) puede planearse desde el decision log.
