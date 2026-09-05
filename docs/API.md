# API PYMZA — Referencia

Backend: Axum 0.6, sirve en `http://127.0.0.1:3000`.

Base URL: `http://127.0.0.1:3000`

Formato de intercambio: `application/json`.

Colecciones Mongo usadas por los endpoints: `empresas`, `clientes`, `planes_pago`, `pagos`, `dashboard_stats`, `verificaciones`.

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
  "password": "demo1234"
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
    "curp": "GARM980412HDFNRL05",
    "nombre_completo": "María García Rodríguez",
    "score": 550,
    "nivel_riesgo": "Medio",
    "historial_pagos": "Sin historial en la red",
    "direccion": "Calle 5 de Mayo 123, CDMX",
    "telefono": "5512345678",
    "correo": "maria@correo.mx",
    "telefono_verificado": false
  }
}
```

`telefono_verificado` siempre se devuelve (`false` para clientes dados de alta
antes de la verificación por OTP, o aún sin verificar). `correo` solo aparece
si el cliente tiene uno.

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

Alta de un cliente nuevo. Valida la CURP de forma robusta: 18 caracteres con
estructura CURP (mayúsculas/dígitos, fecha coherente con el calendario
—incluidos años bisiestos—, sexo, entidad federativa) y **dígito verificador
oficial** (Instructivo RENAPO, DOF 18-10-2021). Evita duplicados. Si viene
`correo`, valida su formato. El score base es `550`, el nivel de riesgo
`"Medio"` y el cliente se crea siempre con `telefono_verificado: false`
(la verificación se hace después por OTP; ver `/api/verificaciones`).

**Requiere:** `Authorization: Bearer <token>`

**Payload:**
```json
{
  "curp": "GARM980412HDFNRL05",
  "nombre_completo": "María García Rodríguez",
  "direccion": "Calle 5 de Mayo 123, CDMX",
  "telefono": "5512345678",
  "correo": "maria@correo.mx"
}
```

`correo` es opcional; si no viene, omítelo o mándalo `null`.

**Respuesta (éxito):**
```json
{
  "status": "success",
  "cliente": {
    "curp": "GARM980412HDFNRL05",
    "nombre_completo": "María García Rodríguez",
    "score": 550,
    "nivel_riesgo": "Medio",
    "historial_pagos": "Sin historial en la red",
    "direccion": "Calle 5 de Mayo 123, CDMX",
    "telefono": "5512345678",
    "telefono_verificado": false
  }
}
```

**Respuestas (error):** CURP inválida (formato o dígito verificador) / correo
inválido / duplicado (mensajes descriptivos), `401` sin token.

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
{ "status": "success", "plan_id": "66c9f2e4a1b2c3d4e5f60718" }
```

`plan_id` es el hex del ObjectId insertado en `planes_pago`; el frontend lo usa
para registrar pagos (también se expone como `_id` en `GET /api/creditos`).

**Respuesta (error al guardar el plan de pago):**
```json
{ "status": "error", "message": "Error al guardar el plan de pago" }
```

**Colecciones Mongo:** `planes_pago` (inserta, con `estado` = `"Activo"` y `fecha` del día) y `dashboard_stats` (upsert por `empresa`, recalculado desde la cartera real: `creditos_activos` = planes Activo o Moroso, `capital_prestado` = suma de `monto_total` de todos los planes, `proximos_cobros` = cuotas que vencen en ≤30 días de planes no liquidados).

---

## POST `/api/creditos/pagos` — protegida

Registra el pago de una cuota de un plan (ola 4). Inserta en `pagos`,
recalcula el estado del plan (`Activo` → `Moroso` si hay cuota vencida sin
pagar → `Liquidado` cuando se pagan todas las cuotas) y devuelve el plan
actualizado con su avance.

**Requiere:** `Authorization: Bearer <token>` — el plan se busca entre los de
la empresa del token; el tenant sale del token, nunca del body.

**Payload:**
```json
{
  "plan_id": "66c9f2e4a1b2c3d4e5f60718",
  "cuota": 1,
  "monto": 1766.67
}
```

`plan_id` = hex del ObjectId del plan (lo expone `GET /api/creditos`).

**Validaciones (en orden):**
1. El plan existe y pertenece a la empresa del token → si no, `404`
   ```json
   { "status": "error", "message": "Plan no encontrado" }
   ```
2. `cuota` en `1..=plazo_meses` → si no, `400`
   ```json
   { "status": "error", "message": "Cuota fuera de rango: debe estar entre 1 y 6" }
   ```
3. La cuota no está pagada ya → si lo está, `400`
   ```json
   { "status": "error", "message": "Cuota ya registrada" }
   ```
4. `monto` igual al `pago_mensual` del plan (tolerancia 1 centavo) → si no, `400`
   ```json
   { "status": "error", "message": "El monto debe ser igual al pago mensual del plan ($1766.67)" }
   ```

**Respuesta (éxito):**
```json
{
  "status": "success",
  "plan": {
    "_id": "66c9f2e4a1b2c3d4e5f60718",
    "empresa": "demo@pymza.mx",
    "cliente_curp": "GARM980412HDFNRL08",
    "producto": "Crédito comercial",
    "monto_total": 10600.0,
    "plazo_meses": 6,
    "pago_mensual": 1766.67,
    "tasa_interes": 0.06,
    "estado": "Activo",
    "fecha": "2026-07-22",
    "cuotas_pagadas": 1,
    "cuotas_vencidas": 0
  }
}
```

**Colecciones Mongo:** `pagos` (inserta `{ plan_id, empresa, cliente_curp, cuota, monto, fecha }`, fecha UTC "YYYY-MM-DD"), `planes_pago` (actualiza `estado` si cambió) y `dashboard_stats` (upsert recalculado).

---

## GET `/api/creditos` — protegida

Lista los créditos (planes de pago) activos de la empresa autenticada.

**Requiere:** `Authorization: Bearer <token>` — los créditos se filtran por `empresa = <correo del token>`; la empresa se lee del token, ya no de la URL.

**Respuesta (éxito):**
```json
{
  "status": "success",
  "creditos": [
    {
      "_id": "66c9f2e4a1b2c3d4e5f60718",
      "empresa": "demo@pymza.mx",
      "cliente_curp": "GARM980412HDFNRL08",
      "producto": "Crédito comercial",
      "monto_total": 10600.0,
      "plazo_meses": 6,
      "pago_mensual": 1766.67,
      "tasa_interes": 0.06,
      "estado": "Activo",
      "fecha": "2026-07-22",
      "cuotas_pagadas": 2,
      "cuotas_vencidas": 0
    }
  ]
}
```

Ola 4: cada crédito expone `_id` (hex, para registrar pagos) y el avance
calculado en servidor — `cuotas_pagadas` (pagos registrados del plan) y
`cuotas_vencidas` (cuotas con vencimiento anterior a hoy sin pago). Estos dos
campos se calculan, no se persisten.

**Colección Mongo:** `planes_pago` y `pagos` (leídos por `empresa` para calcular el avance).

---

## GET `/api/creditos/resumen` — protegida

Resumen de cartera del tenant para las gráficas del dashboard (ola 4). Se
calcula en memoria sobre los planes y pagos de la empresa del token.

**Requiere:** `Authorization: Bearer <token>` — el resumen sale del tenant
del token; los datos nunca cruzan entre empresas.

**Respuesta (éxito):**
```json
{
  "status": "success",
  "resumen": {
    "cobrado_vs_por_cobrar": [
      { "mes": "2026-04", "cobrado": 0.0, "por_cobrar": 0.0 },
      { "mes": "2026-09", "cobrado": 1766.67, "por_cobrar": 3533.34 }
    ],
    "tasa_morosidad": 0.25,
    "flujo_proyectado": [
      { "horizonte": 30, "monto": 1766.67 },
      { "horizonte": 60, "monto": 3533.34 },
      { "horizonte": 90, "monto": 5300.01 }
    ],
    "aging": [
      { "bucket": "0-30", "monto": 1766.67 },
      { "bucket": "31-60", "monto": 0.0 },
      { "bucket": "61-90", "monto": 0.0 },
      { "bucket": "90+", "monto": 0.0 }
    ],
    "top_deudores": [
      { "cliente_curp": "GARM980412HDFNRL08", "nombre": "María García", "saldo": 8833.35 }
    ],
    "distribucion_montos": [
      { "bucket": "0-1k", "n": 0 },
      { "bucket": "1k-5k", "n": 0 },
      { "bucket": "5k+", "n": 1 }
    ]
  }
}
```

Definiciones exactas:
- `cobrado_vs_por_cobrar` — 6 meses: el actual + 5 previos, ascendente
  (`mes` = "YYYY-MM"). `cobrado` = pagos registrados del mes; `por_cobrar` =
  cuotas esperadas de ese mes (vencimiento en el mes, sin pago) en planes no
  liquidados.
- `tasa_morosidad` — f64 0..1 = planes Moroso / planes no liquidados (0 si no
  hay planes no liquidados).
- `flujo_proyectado` — monto de las cuotas que vencen en ≤30 / ≤60 / ≤90 días
  (ventanas acumulativas, hoy incluido) de planes Activo o Moroso.
- `aging` — saldo vencido por antigüedad de la cuota impaga (días desde su
  vencimiento): 1–30 → "0-30", 31–60, 61–90, >90 → "90+".
- `top_deudores` — máx 10, saldo = pago_mensual × plazo − pagos registrados,
  descendente; `nombre` viene de `clientes` por `curp` (si el cliente ya no
  existe, el curp hace de nombre).
- `distribucion_montos` — nº de planes por `monto_total`: <1000 → "0-1k",
  <5000 → "1k-5k", ≥5000 → "5k+".

**Colecciones Mongo:** `planes_pago`, `pagos` y `clientes` (solo lectura).

---

## GET `/api/dashboard` — protegida

Estadísticas del dashboard de la empresa autenticada.

**Requiere:** `Authorization: Bearer <token>` — las stats se filtran por `empresa = <correo del token>`; la empresa se lee del token, ya no de la URL.

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

## POST `/api/verificaciones/solicitar` — protegida

Ola 3 — verificación de teléfono por OTP. Genera un código de 6 dígitos
ligado al par `curp+telefono`, guarda el desafío en la colección
`verificaciones` (**solo el hash SHA-256 del código, nunca en claro**;
expira en 10 minutos; un desafío previo vigente del mismo par se reemplaza)
y lo envía por el `OtpSender` activo:

- **Mock (default en dev):** el código queda impreso en el log del backend
  (`OTP MOCK para <telefono>: <codigo>`).
- **WhatsApp Cloud API (ola 4):** activa si `WHATSAPP_TOKEN` y
  `WHATSAPP_PHONE_NUMBER_ID` existen y no están vacías (ver `.env.example`).
  El envío va por **plantilla de autenticación** (fuera de la ventana de 24 h
  Meta solo permite plantillas): `template.name` = `WHATSAPP_TEMPLATE`
  (default `pymza_otp_verification`), `language.code` =
  `WHATSAPP_TEMPLATE_LANG` (default `es`) y el código como parámetro `text`
  del body. Si el envío falla, el backend solo lo registra en el log y el
  flujo continúa (se puede pedir otro código).

La colección `verificaciones` tiene un **índice TTL** sobre `expira_en`
(BSON date, `expireAfterSeconds: 0`, creado idempotentemente al arrancar el
backend): Mongo borra los desafíos vencidos automáticamente.

**Requiere:** `Authorization: Bearer <token>`

**Payload:**
```json
{
  "curp": "GACM940101HDFRRR09",
  "telefono": "5512345678"
}
```

**Respuesta (éxito):**
```json
{ "status": "success" }
```

**Respuesta (error de DB):** `500` con `{ "status": "error", "message": "Error interno" }`.

**Colección Mongo:** `verificaciones` (documentos `{ curp, telefono, codigo_hash, expira_en }`; `expira_en` es BSON date desde la ola 4, con índice TTL).

---

## POST `/api/verificaciones/confirmar` — protegida

Confirma la verificación del teléfono: valida el código contra el desafío
vigente (no expirado); si coincide, marca `telefono_verificado = true` en el
cliente (actualización de un solo campo), borra el desafío y responde.

**Requiere:** `Authorization: Bearer <token>`

**Payload:**
```json
{
  "curp": "GACM940101HDFRRR09",
  "telefono": "5512345678",
  "codigo": "123456"
}
```

**Respuesta (éxito):**
```json
{ "status": "success", "telefono_verificado": true }
```

**Respuestas (error):**
- `400` — código incorrecto o desafío expirado:
  ```json
  { "status": "error", "message": "Código inválido o expirado" }
  ```
- `404` — no hay desafío para ese `curp+telefono`:
  ```json
  { "status": "error", "message": "No hay un código de verificación solicitado" }
  ```
- `404` — el cliente no existe:
  ```json
  { "status": "error", "message": "Cliente no existe en la red PYMZA" }
  ```

**Colecciones Mongo:** `verificaciones` (lee y borra el desafío) y `clientes`
(actualiza `telefono_verificado`).

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