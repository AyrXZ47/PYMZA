# API PYMZA — Referencia

Backend: Axum 0.6, sirve en `http://127.0.0.1:3000`.

Base URL: `http://127.0.0.1:3000`

Formato de intercambio: `application/json`.

Colecciones Mongo usadas por los endpoints: `empresas`, `clientes`, `planes_pago`, `dashboard_stats`.

## Autenticación (JWT Bearer)

El login devuelve un **JWT real** (HS256, firmado con `JWT_SECRET`, caducidad 24h)
con claims `sub=<correo>`, `nombre=<nombre_empresa>` y `exp` (timestamp unix).

- Todas las rutas salvo `POST /api/login` y `POST /api/empresas` **requieren** el
  header `Authorization: Bearer <token>`.
- Si el token falta, es inválido, está malformado o expirado → `401`:
  ```json
  {
    "status": "error",
    "message": "No autorizado: token JWT ausente, inválido o expirado"
  }
  ```
- El **tenant (empresa) se deriva del token** (`sub` = correo de la empresa),
  nunca de path parameters ni del body. Los documentos de `planes_pago` y
  `dashboard_stats` guardan `empresa: <correo>`.
- `JWT_SECRET` es una variable de entorno obligatoria (`backend/.env`, ver
  `.env.example`); el backend falla al arrancar con mensaje claro si falta.
- Datos previos al aislamiento multi-tenant (empresa = nombre comercial) se
  migran con `backend/scripts/migrate_tenant.js` (idempotente).

---

## POST `/api/login` — pública

Autentica una empresa (correo + password) y devuelve un JWT real.

**Payload:**
```json
{
  "correo": "demo@pymza.mx",
  "password": "demo123"
}
```

**Respuesta (éxito):**
```json
{
  "status": "success",
  "empresa": "Ferretería El Tornillo",
  "token": "<jwt-generado>" 
}
```

**Respuesta (credenciales inválidas o error de DB):**
```json
{
  "status": "error",
  "message": "Credenciales inválidas"
}
```

**Colección Mongo:** `empresas` (busca por `correo` + verifica el hash argon2id de `password`).

---

## POST `/api/empresas` — pública

Alta de una empresa nueva (registro). Valida correo (1 `@`, dominio con punto, sin espacios) y contraseña de al menos 8 caracteres; rechaza correos duplicados.

**Payload:**
```json
{
  "correo": "nueva@pymza.mx",
  "password": "clave1234",
  "nombre_empresa": "Empresa Nueva S.A. de C.V."
}
```

**Respuesta (éxito):**
```json
{
  "status": "success",
  "empresa": {
    "correo": "nueva@pymza.mx",
    "nombre_empresa": "Empresa Nueva S.A. de C.V."
  }
}
```

> La alta no devuelve token: el flujo de registro termina en el login.

**Respuestas (error):**
```json
{ "status": "error", "message": "Correo inválido" }
```

```json
{ "status": "error", "message": "La contraseña debe tener al menos 8 caracteres" }
```

```json
{ "status": "error", "message": "Ya existe una empresa registrada con ese correo" }
```

**Colección Mongo:** `empresas` (inserta; la respuesta no incluye la contraseña).

---

## GET `/api/clientes/:curp` — protegida

Busca un cliente existente en la red PYMZA por su CURP.

**Requiere:** `Authorization: Bearer <token>`

**Parámetro de ruta:** `:curp` — CURP de 18 caracteres.

**Respuesta (encontrado):**
```json
{
  "status": "success",
  "cliente": {
    "curp": "GARM980412HDFNRL08",
    "nombre_completo": "María García Rodríguez",
    "score": 550,
    "nivel_riesgo": "Medio",
    "historial_pagos": "Sin historial en la red",
    "direccion": "Calle 5 de Mayo 123, CDMX",
    "telefono": "5512345678"
  }
}
```

**Respuesta (no existe):**
```json
{
  "status": "not_found",
  "message": "Cliente no existe en la red PYMZA"
}
```

**Colección Mongo:** `clientes` (busca por `curp`).

---

## POST `/api/clientes` — protegida

Alta de un cliente nuevo. Valida el formato de CURP (18 caracteres alfanuméricos) y evita duplicados. El score base es `550` y el nivel de riesgo `"Medio"`.

**Requiere:** `Authorization: Bearer <token>`

**Payload:**
```json
{
  "curp": "GARM980412HDFNRL08",
  "nombre_completo": "María García Rodríguez",
  "direccion": "Calle 5 de Mayo 123, CDMX",
  "telefono": "5512345678"
}
```

**Respuesta (éxito):**
```json
{
  "status": "success",
  "cliente": {
    "curp": "GARM980412HDFNRL08",
    "nombre_completo": "María García Rodríguez",
    "score": 550,
    "nivel_riesgo": "Medio",
    "historial_pagos": "Sin historial en la red",
    "direccion": "Calle 5 de Mayo 123, CDMX",
    "telefono": "5512345678"
  }
}
```

**Respuestas (error):** CURP inválida / duplicado (mensajes descriptivos), `401` sin token.

**Colección Mongo:** `clientes` (inserta).

---

## POST `/api/clientes/:curp/reportar` — protegida

Reporta morosidad de un cliente a la red PYMZA (alerta temprana). Marca al cliente con la alerta; la busca `GET /api/clientes/:curp` posterior la devuelve en el campo `alerta`.

**Requiere:** `Authorization: Bearer <token>` — la empresa que reporta sale del token (`alerta.empresa = <correo>`), ya no se envía en el body.

**Parámetro de ruta:** `:curp` — CURP de 18 caracteres.

**Payload:**
```json
{
  "motivo": "Desapareció con deuda pendiente"
}
```

**Respuesta (éxito):**
```json
{
  "status": "success",
  "alerta": {
    "empresa": "demo@pymza.mx",
    "motivo": "Desapareció con deuda pendiente"
  }
}
```

**Respuesta (motivo vacío):**
```json
{
  "status": "error",
  "message": "Motivo es obligatorio"
}
```

**Respuesta (cliente inexistente):**
```json
{
  "status": "not_found",
  "message": "Cliente no existe en la red PYMZA"
}
```

**Colección Mongo:** `clientes` (actualiza el campo `alerta`).

---

## POST `/api/creditos/evaluar` — protegida

Evalúa un crédito: tasa según plazo (3m=3%, 6m=6%, 9m=10%, 12m=15%, otro=5%), aprueba/rechaza por capacidad de pago y construye el plan de pagos.

**Requiere:** `Authorization: Bearer <token>`

**Payload:**
```json
{
  "curp": "GARM980412HDFNRL08",
  "monto": 10000.0,
  "plazo_meses": 6
}
```

**Respuesta (éxito):**
```json
{
  "status": "success",
  "estado": "Aprobado",
  "pago_mensual": 1766.67,
  "tasa_interes": 0.06,
  "plan_pagos": [
    {
      "mes": 1,
      "pago": 1766.67,
      "interes": 100.0,
      "capital": 1666.67,
      "saldo_restante": 8333.33
    }
  ],
  "consideraciones": "Crédito APROBADO.\n..."
}
```

La capacidad de pago es `$5000.00` mensual si el score del cliente es mayor a 700, o `$2000.00` en caso contrario. Si el pago mensual excede la capacidad, `estado` es `"Rechazado"`.

**Respuesta (cliente no existe):**
```json
{ "status": "error", "message": "Cliente no encontrado" }
```

**Colección Mongo:** `clientes` (solo lectura, por `curp`). No inserta nada.

---

## POST `/api/creditos/autorizar` — protegida

Autoriza un crédito ya evaluado: inserta el plan de pago y actualiza (upsert) las estadísticas del dashboard.

**Requiere:** `Authorization: Bearer <token>` — la empresa sale del token (`planes_pago.empresa` / `dashboard_stats.empresa` = `<correo>`), ya no se envía en el body.

**Payload:**
```json
{
  "cliente_curp": "GARM980412HDFNRL08",
  "producto": "Crédito comercial",
  "monto_total": 10600.0,
  "plazo_meses": 6,
  "pago_mensual": 1766.67,
  "tasa_interes": 0.06
}
```

**Respuesta (éxito):**
```json
{ "status": "success" }
```

**Respuesta (error al guardar el plan de pago):**
```json
{ "status": "error", "message": "Error al guardar el plan de pago" }
```

**Colecciones Mongo:** `planes_pago` (inserta, con `estado` = `"Activo"` y `fecha` del día) y `dashboard_stats` (upsert por `empresa` con `$inc` en `creditos_activos`, `capital_prestado`, `proximos_cobros`).

---

## GET `/api/creditos` — protegida

Lista los créditos (planes de pago) activos de la empresa autenticada.

**Requiere:** `Authorization: Bearer <token>` — los créditos se filtran por `empresa = <correo del token>`; ya no hay path param `:empresa`.

**Respuesta (éxito):**
```json
{
  "status": "success",
  "creditos": [
    {
      "empresa": "demo@pymza.mx",
      "cliente_curp": "GARM980412HDFNRL08",
      "producto": "Crédito comercial",
      "monto_total": 10600.0,
      "plazo_meses": 6,
      "pago_mensual": 1766.67,
      "tasa_interes": 0.06,
      "estado": "Activo",
      "fecha": "2026-07-22"
    }
  ]
}
```

**Colección Mongo:** `planes_pago` (busca por `empresa`).

---

## GET `/api/dashboard` — protegida

Estadísticas del dashboard de la empresa autenticada.

**Requiere:** `Authorization: Bearer <token>` — las stats se filtran por `empresa = <correo del token>`; ya no hay path param `:empresa`.

**Respuesta (con datos):**
```json
{
  "status": "success",
  "stats": {
    "empresa": "demo@pymza.mx",
    "creditos_activos": 1,
    "capital_prestado": 10600.0,
    "proximos_cobros": 6
  }
}
```

**Respuesta (sin registro previo — devuelve ceros):**
```json
{
  "status": "success",
  "stats": {
    "empresa": "demo@pymza.mx",
    "creditos_activos": 0,
    "capital_prestado": 0.0,
    "proximos_cobros": 0
  }
}
```

**Colección Mongo:** `dashboard_stats` (busca por `empresa`).

---

## POST `/api/ocr` — protegida

Validación OCR (placeholder). Devuelve una respuesta fija, no toca la base.

**Requiere:** `Authorization: Bearer <token>`

**Payload:** ninguno (no se lee).

**Respuesta:**
```json
{ "status": "success", "id": "12345" }
```

**Colección Mongo:** ninguna.