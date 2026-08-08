# API PYMZA — Referencia

Backend: Axum 0.6, sirve en `http://127.0.0.1:3000`.

Base URL: `http://127.0.0.1:3000`

Formato de intercambio: `application/json`.

Colecciones Mongo usadas por los endpoints: `empresas`, `clientes`, `planes_pago`, `dashboard_stats`.

> Autenticación: no hay tokens reales. El login devuelve un token estático `"token-temporal-123"` y las demás rutas no lo validan.

---

## POST `/api/login`

Autentica una empresa (correo + password).

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
  "empresa": "Empresa Demo S.A. de C.V.",
  "token": "token-temporal-123"
}
```

**Respuesta (credenciales inválidas o error de DB):**
```json
{
  "status": "error",
  "message": "Credenciales inválidas"
}
```

**Colección Mongo:** `empresas` (busca por `correo` + `password`).

---

## POST `/api/empresas`

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

**Respuestas (error):**
```json
{
  "status": "error",
  "message": "Correo inválido"
}
```

```json
{
  "status": "error",
  "message": "La contraseña debe tener al menos 8 caracteres"
}
```

```json
{
  "status": "error",
  "message": "Ya existe una empresa registrada con ese correo"
}
```

**Colección Mongo:** `empresas` (inserta; la respuesta no incluye la contraseña).

---

## GET `/api/clientes/:curp`

Busca un cliente existente en la red PYMZA por su CURP.

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

**Respuesta (error de DB):**
```json
{
  "status": "error"
}
```

**Colección Mongo:** `clientes` (busca por `curp`).

---

## POST `/api/clientes`

Alta de un cliente nuevo. Valida el formato de CURP (18 caracteres alfanuméricos) y evita duplicados. El score base es `550` y el nivel de riesgo `"Medio"`.

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

**Respuesta (CURP inválida):**
```json
{
  "status": "error",
  "message": "CURP inválida: deben ser 18 caracteres alfanuméricos"
}
```

**Respuesta (duplicado):**
```json
{
  "status": "error",
  "message": "Cliente ya existe en la red PYMZA"
}
```

**Colección Mongo:** `clientes` (inserta).

---

## POST `/api/ocr`

Validación OCR (placeholder). Devuelve una respuesta fija, no toca la base.

**Payload:** ninguno (no se lee).

**Respuesta:**
```json
{
  "status": "success",
  "id": "12345"
}
```

**Colección Mongo:** ninguna.

---

## POST `/api/creditos/evaluar`

Evalúa un crédito: tasa según plazo (3m=3%, 6m=6%, 9m=10%, 12m=15%, otro=5%), aprueba/rechaza por capacidad de pago y construye el plan de pagos.

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
  "consideraciones": "Crédito APROBADO.\nMonto solicitado: $10000.00\nPlazo: 6 meses\nTasa de interés: 6%\nTotal a pagar: $10600.00\nPago mensual: $1766.67\n\nEl cliente tiene capacidad de pago suficiente."
}
```

La capacidad de pago es `$5000.00` mensual si el score del cliente es mayor a 700, o `$2000.00` en caso contrario. Si el pago mensual excede la capacidad, `estado` es `"Rechazado"`.

**Respuesta (cliente no existe):**
```json
{
  "status": "error",
  "message": "Cliente no encontrado"
}
```

**Colección Mongo:** `clientes` (solo lectura, por `curp`). No inserta nada.

---

## POST `/api/creditos/autorizar`

Autoriza un crédito ya evaluado: inserta el plan de pago y actualiza (upsert) las estadísticas del dashboard.

**Payload:**
```json
{
  "empresa": "Empresa Demo S.A. de C.V.",
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
{
  "status": "success"
}
```

**Respuesta (error al guardar el plan de pago):**
```json
{
  "status": "error",
  "message": "Error al guardar el plan de pago"
}
```

**Colecciones Mongo:** `planes_pago` (inserta, con `estado` = `"Activo"` y `fecha` fija) y `dashboard_stats` (upsert por `empresa` con `$inc` en `creditos_activos`, `capital_prestado`, `proximos_cobros`).

---

## GET `/api/creditos/:empresa`

Lista los créditos (planes de pago) activos de una empresa.

**Parámetro de ruta:** `:empresa` — nombre de la empresa.

**Respuesta (éxito):**
```json
{
  "status": "success",
  "creditos": [
    {
      "empresa": "Empresa Demo S.A. de C.V.",
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

**Respuesta (error de DB):**
```json
{
  "status": "error"
}
```

**Colección Mongo:** `planes_pago` (busca por `empresa`).

---

## GET `/api/dashboard/:empresa`

Estadísticas del dashboard de una empresa.

**Parámetro de ruta:** `:empresa` — nombre de la empresa.

**Respuesta (con datos):**
```json
{
  "status": "success",
  "stats": {
    "empresa": "Empresa Demo S.A. de C.V.",
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
    "empresa": "Empresa Demo S.A. de C.V.",
    "creditos_activos": 0,
    "capital_prestado": 0.0,
    "proximos_cobros": 0
  }
}
```

**Colección Mongo:** `dashboard_stats` (busca por `empresa`).
