# API PYMZA — Referencia

Backend: Axum 0.6, sirve en `http://127.0.0.1:3000`.

Base URL: `http://127.0.0.1:3000`

Formato de intercambio: `application/json`.

Colecciones Mongo usadas por los endpoints: `empresas`, `clientes`, `planes_pago`, `pagos`, `dashboard_stats`, `verificaciones`, `recibos`.

## Autenticación (JWT Bearer)

El login devuelve un **JWT real** (HS256, firmado con `JWT_SECRET`, caducidad 24h)
con claims `sub=<correo>`, `nombre=<nombre_empresa>` y `exp` (timestamp unix).

- Todas las rutas salvo `POST /api/login`, `POST /api/empresas` y
  `GET /api/novedades` **requieren** el header `Authorization: Bearer <token>`.
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

## CORS (ola 6)

Los orígenes permitidos se toman de la env **`ALLOWED_ORIGINS`** (separada por
comas): en producción va el dominio público del frontend. Si la env falta o
queda vacía, se usan los de dev — `http://localhost:8080,
http://127.0.0.1:8080` — sin cambio de comportamiento local. Métodos: `GET` /
`POST`; headers: `*`. Un origen con caracteres inválidos para un header se
descarta, no rompe el arranque.

## Rate limit — rutas públicas (ola 6)

`POST /api/login`, `POST /api/empresas` y `GET /api/novedades` tienen un límite
por IP (`tower-governor`): **10 peticiones por segundo con ráfaga de 20** (defaults),
configurable por env `RATE_LIMIT_RPS` / `RATE_LIMIT_BURST` (una env vacía,
inválida o `0` usa el default). Las rutas protegidas NO lo tienen: la sesión
JWT ya las blinda contra el brute-force.

Al exceder el límite, la misma IP recibe `429` con el JSON del contrato:

```json
{ "status": "error", "message": "Demasiadas peticiones, intenta más tarde" }
```

y el header `x-ratelimit-after: <segundos>` indica cuándo vuelve a haber permiso.

## Límite de tamaño de body (ola 6)

El backend acepta requests de hasta **3 MB** (`DefaultBodyLimit` global — antes
2 MB). Es una red de seguridad: el b64 de una imagen de ≤2 MB (~2.7 MB en base64)
llega a los handlers de KYC/recibos, que son quienes rechazan los archivos
demasiado grandes con un `400` y el mensaje del contrato ("El archivo excede el
máximo de 2 MB") — ya no un `413` crudo del framework.

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
    "telefono_verificado": false,
    "ine_verificada": false
  }
}
```

`telefono_verificado` e `ine_verificada` siempre se devuelven (`false` para
clientes dados de alta antes de la verificación por OTP / KYC, o aún sin
verificar). `correo` solo aparece si el cliente tiene uno.

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
    "telefono_verificado": false,
    "ine_verificada": false
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

## POST `/api/clientes/:curp/kyc` — protegida

Ola 5 — KYC de la INE: valida que la imagen subida sea legible (OCR con el
binario `tesseract`, timeout 30 s) y que la CURP leída coincida con la del
cliente; si coincide, marca `ine_verificada = true`. **La imagen NO se
guarda** — solo persiste el resultado de la verificación.

**Requiere:** `Authorization: Bearer <token>`

**Payload:** el archivo va en **base64 dentro del JSON** (no multipart):
```json
{
  "archivo_b64": "<png/jpg/webp en base64>",
  "mime": "image/png"
}
```

**Validaciones (en orden):**
1. `mime` ∈ `image/png` / `image/jpeg` / `image/webp` → si no, `400`:
   ```json
   { "status": "error", "message": "Mime no permitido: solo image/png, image/jpeg o image/webp" }
   ```
2. Tamaño decodificado ≤ **2 MB** (medido por el largo del base64, antes de
   decodificar) → si no, `400`:
   ```json
   { "status": "error", "message": "El archivo excede el máximo de 2 MB" }
   ```
3. Base64 válido → si no, `400`:
   ```json
   { "status": "error", "message": "Base64 inválido" }
   ```
4. El cliente existe → si no, `404`:
   ```json
   { "status": "error", "message": "Cliente no existe en la red PYMZA" }
   ```

**Respuesta (éxito):**
```json
{
  "status": "success",
  "curp_ine": "GAML930528HDFLNR05",
  "nombre_ine": "MARIA GOMEZ LOPEZ",
  "coincide": true,
  "ine_verificada": true
}
```

- `curp_ine` / `nombre_ine`: lo que el OCR leyó de la imagen (`null` si no se
  encontró). La CURP se busca tolerando el ruido típico de OCR (espacios,
  saltos de línea, ligaduras entre caracteres; minúsculas se suben).
- `coincide`: `curp_ine` == CURP del path. Solo con `true` se marca
  `ine_verificada`; con CURP distinta la respuesta trae además `message`
  ("La CURP de la INE no coincide con el cliente") y el cliente NO se marca.
- Si no se encontró CURP en el texto → `success` con `curp_ine: null`,
  `coincide: false` y `message` "No se encontró una CURP legible en la
  imagen".
- Si `tesseract` no está instalado en el servidor o falla → `500`:
  ```json
  { "status": "error", "message": "OCR no disponible en este servidor" }
  ```

**Para probar sin una INE real:** `backend/scripts/fixture_ine.png` lleva la
CURP `GAML930528HDFLNR05` del seed y el nombre `MARIA GOMEZ LOPEZ` (con
`coincide: true`).

**Idioma del OCR:** env `OCR_LANG` (default `"spa"`; ver `.env.example`).

**Colecciones Mongo:** `clientes` (lee y actualiza `ine_verificada`).

---

## POST `/api/clientes/:curp/recibos` — protegida

Ola 5 — score alternativo por recibos de servicios: sube un recibo (luz,
agua o teléfono), el OCR extrae el monto y, si el recibo es legible (monto
encontrado o texto ≥ 50 caracteres), suma **+25** al score del cliente y
recalcula el nivel de riesgo (`score >= 750 → "Bajo"`, `>= 550 → "Medio"`,
`< 550 → "Alto"`). Máximo **2 recibos por cliente** (global por CURP). La
imagen NO se guarda — solo `{curp, empresa, tipo, monto_leido, fecha}`.

**Requiere:** `Authorization: Bearer <token>` — `recibos.empresa` = correo del
token (quién subió); el tope de 2 es global por cliente, sin importar la
empresa que suba.

**Payload:**
```json
{
  "archivo_b64": "<png/jpg/webp en base64>",
  "mime": "image/png",
  "tipo": "luz"
}
```

**Validaciones:** `tipo` ∈ `luz` / `agua` / `telefono` → si no, `400` ("Tipo
inválido: debe ser luz, agua o telefono"); después mime / tamaño (2 MB) /
base64 / cliente-existe igual que en KYC.

**Respuesta (éxito):**
```json
{
  "status": "success",
  "monto_leido": 450.0,
  "score": 575,
  "nivel_riesgo": "Medio",
  "recibos_contados": 1
}
```

- `monto_leido`: el monto que el OCR extrajo (`null` si no encontró ninguno;
  se leen formatos como `$1,234.56`, `1234.56 MXN`, `TOTAL: $450.00`).
- Si el recibo NO es legible (sin monto y texto < 50 chars): `success` con el
  score SIN cambio, el recibo no se guarda y `recibos_contados` refleja los
  que ya tenía el cliente.
- Tercer recibo (con 2 ya guardados y recibo legible) → `400`:
  ```json
  { "status": "error", "message": "Máximo 2 recibos por cliente" }
  ```
- tesseract ausente/fallido → `500` "OCR no disponible en este servidor"
  (igual que KYC).

**Colecciones Mongo:** `recibos` (inserta y cuenta por `curp`), `clientes`
(actualiza `score` y `nivel_riesgo`).

---

## POST `/api/creditos/evaluar` — protegida

Evalúa un crédito: tasa según plazo (`1m=7%, 3m=9%, 6m=12%, 9m=15%, 12m=18%`), aprueba/rechaza por capacidad de pago y construye el plan de pagos.

**Requiere:** `Authorization: Bearer <token>`

**Payload:**
```json
{
  "curp": "GARM980412HDFNRL08",
  "monto": 10000.0,
  "plazo_meses": 6
}
```

`plazo_meses` debe ser **1, 3, 6, 9 o 12**; cualquier otro valor devuelve `400` con
`"El plazo debe ser 1, 3, 6, 9 o 12 meses"`. `monto` debe ser un número finito
mayor a 0 (`400` `"El monto debe ser mayor a 0"` en caso contrario).

**Respuesta (éxito):**
```json
{
  "status": "success",
  "estado": "Aprobado",
  "pago_mensual": 1766.67,
  "tasa_interes": 0.12,
  "plan_pagos": [
    {
      "mes": 1,
      "pago": 1766.67,
      "interes": 200.0,
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

**Ola 8 (E2):** `pago_mensual` y `tasa_interes` del body se siguen aceptando por
compatibilidad pero **se ignoran**: el backend recalcula
`tasa = tasa_por_plazo(plazo)` y
`pago_mensual = round(monto_total × (1 + tasa) / plazo, 2)` y persiste esos
valores. La deuda (`pago_mensual × plazo`) queda así determinada por
`monto_total` + `plazo`, nunca por el body.

**Ola 8-fix (E2, tope):** `monto_total` debe ser finito, `> 0` y
`≤ MONTO_MAX_MXN` (1e12 MXN) → si no, `400` (mensaje
`"El monto excede el máximo permitido"`). Además, antes de persistir se verifica
que `pago_mensual_de(...)` sea finito. Con esto nunca se guarda
`pago_mensual: Infinity` en `planes_pago` (antes un monto enorme corrompía el
dashboard del tenant de forma permanente).

**Respuesta (error al guardar el plan de pago):**
```json
{ "status": "error", "message": "Error al guardar el plan de pago" }
```

**Colecciones Mongo:** `planes_pago` (inserta, con `estado` = `"Activo"` y `fecha` del día) y `dashboard_stats` (upsert por `empresa`, recalculado desde la cartera real: `creditos_activos` = planes Activo o Moroso, `capital_prestado` = suma de `monto_total` de todos los planes, `proximos_cobros` = cuotas que vencen en ≤30 días de planes no liquidados). El estado usado es el **recalculado en lectura** (ola 7).

---

## POST `/api/creditos/pagos` — protegida

Registra el pago de una cuota de un plan (ola 4). Inserta en `pagos` con
`tipo: "cuota"`, recalcula el saldo/estado por dinero (ver §"Semántica de saldo
y estado (ola 7)") y devuelve el plan actualizado con su avance, `cobrado` y
`saldo`.

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
    "tasa_interes": 0.12,
    "estado": "Activo",
    "fecha": "2026-07-22",
    "cuotas_pagadas": 1,
    "cuotas_vencidas": 0,
    "cobrado": 1766.67,
    "saldo": 8833.35
  }
}
```

**Colecciones Mongo:** `pagos` (inserta `{ plan_id, empresa, cliente_curp, cuota, monto, fecha, tipo: "cuota" }`, fecha UTC "YYYY-MM-DD"), `planes_pago` (actualiza `estado` si cambió y reserva el contador `cobrado`) y `dashboard_stats` (upsert recalculado).

**Concurrencia (ola 8, E1; recuperable en 8-fix):** antes de insertar, el handler
**reserva de forma atómica** el monto en el contador `cobrado` del plan con un guard
`$expr: cobrado + monto <= pago_mensual × plazo` (`find_one_and_update` + `$inc`
sobre `cobrado` y `reservas`, `$set` de `reserva_ts`; sin transacciones). Si dos
pagos concurrentes compiten por el mismo saldo, los que no caben reciben `400` y
no insertan nada. Si el insert falla (incluido el `E11000` del índice único
parcial de `plan_id+cuota`), se revierte el `$inc` y se devuelve `500`; si el
`Pago` se inserta, se confirma la reserva (`reservas -= 1`). Una reserva que
quede colgada se recupera en la reconciliación (§"Semántica de saldo y estado").

### Semántica de saldo y estado (ola 7)

Todo se calcula por **dinero**, no por cuotas marcadas:

- `deuda = pago_mensual × plazo_meses`.
- `cobrado` = suma de TODOS los pagos (cuotas) y abonos del plan.
- `saldo = max(0, deuda − cobrado)` (2 decimales).
- `cuotas_pagadas` = `min(plazo, floor(cobrado / pago_mensual))` (cuotas cubiertas por dinero).
- `cuotas_vencidas` = cuotas con vencimiento anterior a hoy, menos las cubiertas por dinero (nunca negativas).
- `estado`: `Liquidado` si `saldo <= 0.01`; si no, `Moroso` si `cuotas_vencidas > 0`; si no, `Activo`.

`estado`, `cobrado`, `saldo`, `cuotas_pagadas` y `cuotas_vencidas` se **recalculan en lectura**; el `estado` persistido en `planes_pago` es solo caché. Los pagos guardados antes de esta ola (sin `tipo`) se leen como `"cuota"`, así que el saldo/estado de los planes viejos no cambia.

**Ola 8 (E1):** `planes_pago` guarda además un contador operativo `cobrado`
(campo crudo, no movido por el ledger) que se reserva de forma atómica antes de
cada pago/abono, junto con `reservas` (nº de reservas en vuelo) y `reserva_ts`
(epoch ms de la última reserva). El ledger (`pagos`) sigue siendo la verdad de
auditoría; el contador solo evita que N pagos concurrentes rebasen la deuda.

**Ola 8-fix (E1, recuperación):** el contador dejó de ser irreversible. Al leer
la cartera se **reconcilia en ambas direcciones** (sube o baja) contra la suma
del ledger, con dos salvaguardas:

- Mientras haya reservas **frescas** en vuelo (`reservas > 0` y `reserva_ts`
  reciente), no se toca `cobrado`: bajar el contador clobbearía una reserva
  concurrente aún sin confirmar. Una reserva **muerta** (más de 5 minutos) se
  recupera.
- Los planes legacy sin `reservas` se tratan como 0, así un `cobrado` inflado se
  auto-repara en su primera lectura y el plan vuelve a ser cobrable.

Una reserva se limpia en cuanto el `Pago` queda insertado (`reservas -= 1`) o se
revierte si el insert falla (`cobrado -= monto, reservas -= 1`). Para reparar en
bloque planes legacy con dinero inconsistente existe
`backend/scripts/reparar_planes.js` (dry-run por defecto; lo corre el operador
con `APPLY=1`).

---

## POST `/api/creditos/abonos` — protegida

Ola 7 — registra un **abono parcial** (pago a cuenta) que baja el saldo **sin marcar ninguna cuota como pagada**. Es la vía para "cada semana le abonan, pero no se marca la cuota hasta saldar".

**Requiere:** `Authorization: Bearer <token>` — el plan se busca entre los de la empresa del token; el tenant sale del token, nunca del body.

**Payload:**
```json
{
  "plan_id": "66c9f2e4a1b2c3d4e5f60718",
  "monto": 500.0,
  "nota": "abono semanal"
}
```

`nota` es opcional.

**Ola 8:** `nota` se acota a **280 caracteres** (se recorta, nunca rechaza).

**Validaciones (en orden):**
1. El plan existe y pertenece a la empresa del token → si no, `404`
   ```json
   { "status": "error", "message": "Plan no encontrado" }
   ```
2. `monto` finito y `> 0` → si no, `400`
   ```json
   { "status": "error", "message": "El monto debe ser mayor a 0" }
   ```
3. El plan no está liquidado → si ya lo está, `400`
   ```json
   { "status": "error", "message": "El plan ya está liquidado" }
   ```
4. `monto <= saldo + 0.01` → si no, `400`
   ```json
   { "status": "error", "message": "El abono excede el saldo pendiente" }
   ```

**Concurrencia (ola 8, E1; recuperable en 8-fix):** igual que en
`POST /api/creditos/pagos`, el abono reserva `cobrado`/`reservas` de forma
atómica antes de insertar y confirma o revierte la reserva. N abonos
concurrentes nunca suman más que la deuda: los que no caben reciben `400`.

**Respuesta (éxito):** la misma shape que `POST /api/creditos/pagos` (el plan con `cobrado`/`saldo` recalculados y el `estado` sin cambios si aún hay saldo).

**Colecciones Mongo:** `pagos` (inserta `{ plan_id, empresa, cliente_curp, cuota: 0, monto, fecha, tipo: "abono", nota? }`), `planes_pago` (actualiza `estado` si quedó liquidado y reserva `cobrado`) y `dashboard_stats` (upsert recalculado).

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
      "nombre": "María García Rodríguez",
      "producto": "Crédito comercial",
      "monto_total": 10600.0,
      "plazo_meses": 6,
      "pago_mensual": 1766.67,
      "tasa_interes": 0.12,
      "estado": "Activo",
      "fecha": "2026-07-22",
      "cuotas_pagadas": 1,
      "cuotas_vencidas": 0,
      "cobrado": 1766.67,
      "saldo": 8833.35
    }
  ]
}
```

Ola 4: cada crédito expone `_id` (hex, para registrar pagos). Ola 7 añade:

- `nombre` — nombre completo del cliente, resuelto con **una sola** consulta
  `clientes.find({curp: {$in: [...]}})` (si el cliente ya no está en la red, el
  CURP hace de nombre).
- `cobrado` / `saldo` y `cuotas_pagadas` / `cuotas_vencidas` con la semántica por
  dinero de §"Semántica de saldo y estado (ola 7)".
- `estado` **recalculado en lectura** (nunca el persistido).

**Colección Mongo:** `planes_pago` y `pagos` (leídos por `empresa` para calcular el avance) y `clientes` (un `$in` por los CURPs del tenant para `nombre`).

---

## GET `/api/creditos/:plan_id/contrato` — protegida

Ola 6 — genera y descarga el **PDF del contrato de crédito** del plan. Se
regenera bajo demanda desde datos vivos (nunca se almacena). Contenido: título
"CONTRATO DE CRÉDITO", fecha de emisión, datos de la empresa (nombre, correo),
del cliente (nombre, CURP), datos del crédito (producto, monto, plazo, tasa,
pago mensual), la **tabla completa de pagos** (mes, pago, interés, capital,
saldo — misma fórmula de `evaluar`), línea de firma y leyenda.

Ola 7 añade al PDF una sección de **pagos y abonos registrados** (fecha, tipo,
monto), el `cobrado`/`saldo` al momento de emitirse y el sello
**`LIQUIDADO — FINIQUITO`** cuando `saldo <= 0.01`.

**Requiere:** `Authorization: Bearer <token>` — solo planes del tenant del
token; el plan ajeno no aparece con el filtro por empresa.

**Parámetro de ruta:** `:plan_id` — hex del ObjectId del plan (lo expone
`GET /api/creditos`).

**Respuesta (éxito):** archivo válido
- `Content-Type: application/pdf`
- `Content-Disposition: attachment; filename="contrato-<curp>.pdf"`
- body: bytes del PDF (header `%PDF-`, formato 1.3, A4)

**Respuestas (error):**
- `400` — `plan_id` no es un ObjectId hex:
  ```json
  { "status": "error", "message": "plan_id inválido: se espera el hex del ObjectId" }
  ```
- `404` — plan inexistente o de otra empresa:
  ```json
  { "status": "error", "message": "Plan no encontrado" }
  ```
- `401` sin token — igual que todas las protegidas.

Si el cliente ya no existe en la red, el CURP hace de nombre en el PDF
(patrones de la ola 4); si la empresa no está, el correo.

**Colecciones Mongo:** `planes_pago` (lee por `_id` + `empresa`), `empresas` y
`clientes` (solo lectura, para los nombres del PDF).

---

## GET `/api/creditos/resumen` — protegida

Resumen de cartera del tenant para las gráficas del dashboard (ola 4). Se
calcula en memoria sobre los planes y pagos de la empresa del token.

**Requiere:** `Authorization: Bearer <token>` — el resumen sale del tenant
del token; los datos nunca cruzan entre empresas.

**Query params (ola 8, opcionales):** `desde=YYYY-MM-DD&hasta=YYYY-MM-DD`. Si
ambos son válidos, la serie `cobrado_vs_por_cobrar` usa como buckets los meses
de esa ventana y solo cuenta pagos/cuotas con fecha dentro de ella. Sin ellos se
conservan los 6 meses (actual + 5 previos). El resto de las gráficas es estado
actual y no depende de la ventana.

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
- `cobrado_vs_por_cobrar` — sin ventana: 6 meses (actual + 5 previos,
  ascendente, `mes` = "YYYY-MM"); con `?desde&hasta`: los meses de la ventana.
  `cobrado` = pagos/abonos con fecha dentro de la ventana (o del mes); `por_cobrar`
  = cuotas esperadas de esa ventana (vencimiento en ella) **no cubiertas por
  dinero** (`cuotas_cubiertas`, abonos incluidos) en planes no liquidados.
- `tasa_morosidad` — f64 0..1 **por dinero** (ola 8): `cartera_vencida /
  capital_colocado`, donde vencida = suma de `saldo` de planes Moroso y colocado
  = suma de `monto_total` de planes no liquidados (0 si el capital colocado es 0).
- `flujo_proyectado` — monto de las cuotas **no cubiertas por dinero** que vencen
  en ≤30 / ≤60 / ≤90 días (ventanas acumulativas, hoy incluido) de planes con
  estado recalculado Activo o Moroso.
- `aging` — saldo vencido por antigüedad de la cuota impaga **no cubierta por
  dinero** (días desde su vencimiento): 1–30 → "0-30", 31–60, 61–90, >90 → "90+".
  Solo planes no liquidados. Un abono de 500 sobre una deuda de 3000 (pago 500 ×
  6) deja `aging.90+ = 2500`, no 3000.
- `top_deudores` — máx 10, saldo = pago_mensual × plazo − pagos registrados,
  descendente; `nombre` viene de `clientes` por `curp` (si el cliente ya no
  existe, el curp hace de nombre).
- `distribucion_montos` — nº de planes por `monto_total`: <1000 → "0-1k",
  <5000 → "1k-5k", ≥5000 → "5k+".

**Estado recalculado (ola 8, cierra O1):** todas las particiones (aging,
morosidad, flujo, top, distribución) usan el estado recalculado en lectura, no
el `estado` persistido — un plan liquidado por abonos ya no aparece como moroso.

**Colecciones Mongo:** `planes_pago`, `pagos` y `clientes` (solo lectura).

---

## GET `/api/dashboard` — protegida

Estadísticas del dashboard de la empresa autenticada. **Ola 8:** se calculan en
vivo desde la cartera del tenant (el `dashboard_stats` persistido deja de ser la
fuente; puede seguir escribiéndose por compat).

**Requiere:** `Authorization: Bearer <token>` — las stats se filtran por `empresa = <correo del token>`; la empresa se lee del token, ya no de la URL.

**Query params (ola 8, opcionales):** `desde=YYYY-MM-DD&hasta=YYYY-MM-DD`
(ambos requeridos para formar ventana). `cobrado_periodo` solo suma pagos/abonos
con fecha dentro de la ventana; sin ventana usa el histórico completo.

**Respuesta (con datos):**
```json
{
  "status": "success",
  "stats": {
    "empresa": "demo@pymza.mx",
    "creditos_activos": 1,
    "capital_prestado": 10600.0,
    "proximos_cobros": 6,
    "capital_colocado": 10600.0,
    "cobrado_periodo": 1766.67,
    "por_cobrar_neto": 8833.35,
    "cartera_vencida": 8833.35,
    "tasa_morosidad": 0.8333
  }
}
```

Definiciones (todas calculadas desde `planes_pago` + `pagos` del tenant):
- `capital_colocado` — suma de `monto_total` de planes **no liquidados**.
- `cobrado_periodo` — suma de pagos/abonos con `fecha` dentro de la ventana
  (histórico si no se manda ventana).
- `por_cobrar_neto` — suma de `saldo` de planes no liquidados.
- `cartera_vencida` — suma de `saldo` de planes con estado recalculado `Moroso`.
- `tasa_morosidad` — `cartera_vencida / capital_colocado` (**por dinero**, no por
  número de planes); 0 si no hay capital colocado.
- Los 3 campos viejos se conservan: `creditos_activos` = planes Activo o Moroso,
  `capital_prestado` = suma de `monto_total` de todos los planes,
  `proximos_cobros` = cuotas que vencen en ≤30 días de planes no liquidados.

**Respuesta (sin planes — devuelve ceros):**
```json
{
  "status": "success",
  "stats": {
    "empresa": "demo@pymza.mx",
    "creditos_activos": 0,
    "capital_prestado": 0.0,
    "proximos_cobros": 0,
    "capital_colocado": 0.0,
    "cobrado_periodo": 0.0,
    "por_cobrar_neto": 0.0,
    "cartera_vencida": 0.0,
    "tasa_morosidad": 0.0
  }
}
```

**Colección Mongo:** `planes_pago` y `pagos` (lectura del tenant). `dashboard_stats`
ya no es fuente de estos KPIs.

---

## GET `/api/novedades` — pública (ola 8)

Changelog estático para la campanita "what's new" del frontend. **Sin JWT**
(comparte el rate limit por IP de las rutas públicas) y sin datos de ninguna
empresa.

**Respuesta:**
```json
{
  "status": "success",
  "version": "0.8.0",
  "novedades": [
    {
      "fecha": "2026-10-06",
      "titulo": "Abonos a prueba de concurrencia",
      "detalle": "Los abonos y pagos concurrentes ya no pueden rebasar la deuda…"
    }
  ]
}
```

- `version` — const `VERSION` de `backend/src/novedades.rs`; se bumpea por
  release. El frontend la compara con su `APP_VERSION` compilada: si la del
  servidor es mayor, muestra "hay una actualización disponible — recarga".
- `novedades` — hitos (fecha, título, detalle) ordenados del más reciente al más
  antiguo.

**Colección Mongo:** ninguna.

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