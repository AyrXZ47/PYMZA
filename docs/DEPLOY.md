# Despliegue en Railway — PYMZA

Guía para llevar PYMZA a producción en [Railway](https://railway.app): dos
servicios (backend Axum + frontend WASM servido por nginx) apuntando a un
cluster de **MongoDB Atlas**. Todos los secretos van como variables de
entorno en el panel de Railway — **nunca en el repo** (este documento solo
usa placeholders `<pegar-aquí>`).

## Arquitectura

```
Navegador ──> Frontend (nginx :8080, WASM con API_BASE compilado)
                    │  fetch HTTPS
                    v
             Backend (Axum :3000)  ──>  MongoDB Atlas (MONGODB_URI)
```

- **Backend**: se construye con `Dockerfile.backend` (raíz del repo). Escucha
  en `BIND_ADDR=0.0.0.0:3000`.
- **Frontend**: se construye con `Dockerfile.frontend`. El build arg
  `API_BASE` queda **compilado dentro del WASM** (`option_env!` en
  `frontend/src/api.rs`): cambiarlo exige re-build del servicio.
- **Base de datos**: MongoDB Atlas (backups automáticos incluidos).

## Orden de setup

### 1. Backend

1. Railway → **New Project** → **Deploy from GitHub repo** (raíz del repo;
   Railway detecta los Dockerfiles de la raíz; si pregunta, elegir
   `Dockerfile.backend`).
2. En **Variables** del servicio backend, agrega:

   | Variable | Valor |
   |---|---|
   | `MONGODB_URI` | `<pegar-aquí>` — connection string de Atlas (`mongodb+srv://usuario:password@cluster.mongodb.net/`) |
   | `JWT_SECRET` | `<pegar-aquí>` — cadena aleatoria larga (≥ 32 chars; p. ej. `openssl rand -hex 32`) |
   | `BIND_ADDR` | `0.0.0.0:3000` |
   | `ALLOWED_ORIGINS` | `<pegar-aquí>` — dominio público del frontend (paso 3), separado por comas si hay varios. Sin `/` final |
   | `OCR_LANG` | `spa` |
   | `WHATSAPP_TOKEN` | `<pegar-aquí>` — token de la Cloud API de WhatsApp (opcional: sin él el OTP cae a modo log) |
   | `WHATSAPP_PHONE_NUMBER_ID` | `<pegar-aquí>` |
   | `WHATSAPP_TEMPLATE` | `<pegar-aquí>` |
   | `WHATSAPP_TEMPLATE_LANG` | `es` |
   | `RATE_LIMIT_RPS` | opcional — req/s por IP en rutas públicas (hay default) |
   | `RATE_LIMIT_BURST` | opcional — ráfaga permitida por IP (hay default) |

   > **Importante:** `ALLOWED_ORIGINS` no puede llenarse todavía si el
   > frontend aún no tiene dominio. Déjala con el default dev y vuelta a
   > llenar en el paso 4 (o declara el dominio deseado y despliega el
   > frontend después).
3. En **Settings → Networking → Generate Domain**: crea el dominio público y
   mapea el puerto **3000**. Ese dominio (p. ej.
   `https://pymza-backend.up.railway.app`) es la URL que irá en `API_BASE`.

### 2. Copiar el dominio público del backend

Guárdalo: es el valor del build arg `API_BASE` del frontend (paso 3). Ten
presente la distinción: `ALLOWED_ORIGINS` del backend lleva el dominio del
**frontend**; el dominio del **backend** va en el build arg `API_BASE`.

### 3. Frontend

1. Railway → mismo proyecto → **New Service → GitHub repo** de nuevo
   (segunda instancia del mismo repo); en **Build**, usa
   `Dockerfile.frontend`.
2. En **Settings → Build** agrega la variable de build:

   | Build arg | Valor |
   |---|---|
   | `API_BASE` | `<pegar-aquí>` — el dominio público del backend del paso 2, **con** `https://` y **sin** `/` final |

   (Desde la UI: en el servicio → Settings → Build → "Add build arg", o en
   `railway.toml`.) Compilar sin `API_BASE` produce un WASM que llama a
   `http://127.0.0.1:3000` — inútil en producción.
3. **Settings → Networking → Generate Domain** para el frontend y mapea el
   puerto **8080**.

### 4. Re-deploy si cambió CORS

Si `ALLOWED_ORIGINS` del backend se llenó (o cambió) **después** de conocer
el dominio del frontend: edita la variable y haz **re-deploy del backend**
(el CORS se evalúa por petición, pero la env solo se lee al arrancar). El
frontend no necesita re-build por esto; sí lo necesita si cambió `API_BASE`.

## Verificación post-deploy

```bash
# 1. Login desde el dominio nuevo (debe devolver JSON con token)
curl -s -X POST https://<dominio-backend>/api/login \
  -H 'content-type: application/json' \
  -d '{"correo":"demo@pymza.mx","password":"demo1234"}'

# 2. Sin credenciales válidas → 401 (no 404 ni CORS error)
curl -s -o /dev/null -w "%{http_code}\n" https://<dominio-backend>/api/dashboard
```

3. Abre `https://<dominio-frontend>` en el navegador, haz login y confirma:
   dashboard con datos, alta de cliente y **Descargar contrato** en cartera
   (el PDF baja). Si el login carga pero las llamadas fallan en consola con
   CORS → revisa `ALLOWED_ORIGINS` (paso 4).

## Backups de MongoDB Atlas

Sin código propio: los trae el cluster (backups automáticos/policy del
proveedor).

- **Dónde verlos**: Atlas UI → tu cluster → pestaña **Backup** → lista de
  snapshots por fecha (Continuous Backup / scheduled snapshots).
- **Cómo verificarlos**: confirma que hay un snapshot de las últimas 24 h y
  revisa su tamaño (no 0 B). Una vez, prueba un **restore puntual a un
  cluster temporal** y valida que `clientes`/`planes_pago` tienen documentos.
- La URI de Atlas con la que arranca el backend es la que determina qué
  cluster se respalda — verifica que es la del entorno productivo.

## Rotación de `JWT_SECRET`

- **Efecto inmediato**: rotar la secret invalida **todos** los tokens
  emitidos con la anterior (logout masivo; las sesiones abiertas reciben 401
  y el frontend las manda al login). No hay refresh tokens.
- **Cuándo**: en ventana de mantenimiento, tras comunicarlo.
- **Cómo**: genera una nueva (`openssl rand -hex 32`), reemplaza
  `JWT_SECRET` en Railway y re-deploy del backend. Los usuarios vuelven a
  iniciar sesión; nada más se rompe.

## Troubleshooting

| Síntoma | Causa probable | Fix |
|---|---|---|
| Navegador: llamadas bloqueadas por CORS | `ALLOWED_ORIGINS` no contiene el dominio exacto del frontend (scheme + host, sin `/` final) | Corrige la env y re-deploy del backend (paso 4) |
| Login por curl funciona pero no desde el navegador | `API_BASE` mal puesto en el build del frontend (default `http://127.0.0.1:3000`) o dominio del backend no es https | Re-build del frontend con el `API_BASE` correcto (paso 3) |
| OCR responde 500 | Falta `tesseract`/`tesseract-ocr-spa` en la imagen o el idioma no coincide con `OCR_LANG` | Verifica `Dockerfile.backend`; revisa los logs del contenedor |
| 429 en login/registro | Rate limiting por IP activo en rutas públicas | Espera la ventana o ajusta `RATE_LIMIT_RPS`/`RATE_LIMIT_BURST`; si es masivo, investiga quién golpea el endpoint |
| Backend no arranca | `JWT_SECRET` ausente/vacía (el proceso muere con mensaje claro) o `MONGODB_URI` inválida | Revisa los logs de despliegue y las variables del paso 1 |

## Nota de seguridad

Los valores reales de `MONGODB_URI`, `JWT_SECRET` y `WHATSAPP_TOKEN` viven
solo en el panel cifrado de Railway (o tu gestor de secretos). Jamás en el
repo, issues o capturas.
