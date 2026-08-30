# Brief: Wave 3 · Executor 1 — Backend: identidad verificable (CURP robusta + correo + OTP teléfono)

> Copia de `.workflow/briefs/_template.md`. El executor no toca archivos que no
> posee, ni "obviamente". Desviaciones → decision log en `.workflow/plan.md`.

## Tarea

Implementar EXACTAMENTE el "Contrato API ola 3" de `.workflow/plan.md`:

1. **CURP robusta**: fortalecer `es_curp_valida` en
   `backend/src/routes/cliente.rs` añadiendo el **dígito verificador** (18º
   carácter) y coherencia real de fecha (mes/día válidos según el mes, años
   bisiestos) manteniendo lo ya validado (18 chars, mayúsculas/dígitos, sexo,
   entidad federativa). El algoritmo del dígito verificador CURP es público
   (tabla de valores 0-9/A-Z + factor según posición + módulo 10); deja la
   tabla y el cálculo en una función pura `dígito_verificador(curp) -> char`
   con tests. Las CURP del seed (`RAMJ920215MDFMZR03`, `GAML930528HDFLNR05`,
   `GARV850710MCHLRN09`) deben seguir siendo válidas.
2. **Correo del cliente**: `Cliente` gana `correo: Option<String>` (serde
   default) y `telefono_verificado: bool` (default `false`); `CrearClienteReq`
   gana `correo: Option<String>`. En `crear_cliente`, validar el correo con
   `es_correo_valido` (ya existe en `auth.rs`) si viene `Some`; el cliente se
   crea SIEMPRE con `telefono_verificado: false`.
3. **OTP por teléfono**:
   - `backend/src/otp.rs` (nuevo): función pura `generar_codigo() -> String`
     (6 dígitos, `rand`), `hash_codigo(&str) -> String` (`sha2` SHA-256 hex),
     trait `OtpSender { fn enviar(&self, telefono: &str, codigo: &str) }` con
     `MockOtpSender` (eprintln: "OTP MOCK para {telefono}: {codigo}") y
     `WhatsAppOtpSender` (POST a la WhatsApp Cloud API de Meta vía `reqwest`,
     usando `WHATSAPP_TOKEN` y `WHATSAPP_PHONE_NUMBER_ID` de env; si la
     llamada falla, eprintln sin panickear).
   - `backend/src/routes/verificacion.rs` (nuevo): handlers protegidos con
     `EmpresaSession`:
     - `solicitar_verificacion` — body `{ curp, telefono }`: genera código,
       guarda en colección `verificaciones` el documento `{ curp, telefono,
       codigo_hash, expira_en: now+10min }` (NUNCA el código en claro),
       envía por el `OtpSender` activo, responde `{status: "success"}`. Si ya
       existe un desafío vigente para curp+telefono, reemplazarlo.
     - `confirmar_verificacion` — body `{ curp, telefono, codigo }`: busca el
       desafío vigente (no expirado), compara `hash_codigo(codigo)` con
       `codigo_hash`; si coincide → `telefono_verificado = true` en el
       cliente (update de un solo campo), borra el desafío, responde
       `{status: "success", telefono_verificado: true}`; si el código no
       coincide → 400 `{status:"error", message:"Código inválido o
       expirado"}`; si no hay desafío → 404; si no existe el cliente → 404.
   - `main.rs`: registrar `POST /api/verificaciones/solicitar` y `POST
     /api/verificaciones/confirmar` (protegidas, dentro del Router con
     `with_state`); construir el `OtpSender` activo una vez (mock por
     defecto; `WhatsAppOtpSender` si las dos env `WHATSAPP_*` existen) y
     pasarlo como State (ampliar el State a un struct `AppState { client,
     otp_sender }` o usar el patrón existente de la app — documenta la
     elección con `ponytail:` si simplificas).
   - `backend/src/routes/mod.rs`: exponer `pub mod verificacion;`.
4. **Deps nuevas** (SOLO estas): `reqwest` (features json/rustls-tls o
   default; coherente con OpenSSL ya presente), `rand`, `sha2`.
5. **docs/API.md**: documentar los 2 endpoints nuevos + campos nuevos de
   cliente. **.env.example**: `WHATSAPP_TOKEN=""` y
   `WHATSAPP_PHONE_NUMBER_ID=""`.
6. Tests unitarios: dígito verificador (CURPs válidas conocidas + CURP con
   formato OK pero dígito malo → rechazada), `generar_codigo` (6 dígitos),
   `hash_codigo` (determinista, hex), y round-trip del flujo con mock si es
   factible sin DB (o al menos las funciones puras). No romper los tests
   existentes de `cliente.rs` (CURPS_SEED deben seguir pasando).

Ponytail: sin abstracciones de más — un trait `OtpSender` y dos impls es lo
mínimo que permite intercambiar WhatsApp por otro proveedor sin reescribir la
lógica. Marca con `ponytail:` los atajos deliberados nombrando su techo.

## Definition of done

- CURP con dígito verificador: CURP de formato válido pero dígito malo →
  rechazada (test); CURPs del seed → válidas (test).
- `POST /api/clientes` acepta `correo` opcional y crea cliente con
  `telefono_verificado: false`; rechaza correo con formato inválido.
- `solicitar` guarda solo `codigo_hash` (verificable: `rg "codigo_hash"` en
  `backend/src/` y el doc en DB no contiene el código en claro); responde
  success y el mock imprime el código en el log.
- `confirmar` con código correcto marca `telefono_verificado=true`; con código
  incorrecto → 400; sin desafío → 404; sin cliente → 404.
- Sin token → 401 en los 2 endpoints nuevos (el extractor protege).
- Deps nuevas = solo las 3 listadas. El comando de verify de abajo pasa.

## Files you own

- `backend/Cargo.toml`
- `backend/src/**`
- `backend/scripts/**` (solo si algo del seed necesita el campo nuevo)
- `docs/API.md`
- `.env.example`

## Files forbidden

- `frontend/**`, `AGENTS.md` (raíz), `README.md`, `docs/ROADMAP.md`,
  `docs/INVESTIGACION.md`, `PYMZA.md`, `docker-compose.yml`, `Dockerfile.*`,
  `.workflow/**`, `skills/**`, `backend/.env` (secreto del humano)

## Read first

- `.workflow/plan.md` — sección "Ola 3 (actual)" y su Contrato API.
- `backend/src/main.rs` (wiring y State actual), `backend/src/routes/cliente.rs`
  (validación CURP actual, `crear_cliente`), `backend/src/auth.rs`
  (`EmpresaSession`, `es_correo_valido`), `backend/src/models/cliente.rs`,
  `backend/src/routes/mod.rs`.
- `backend/scripts/seed.js` (CURPs del seed que deben seguir siendo válidas).

## Verify command

```bash
cd backend && cargo build && cargo test
```

## Commit

- MANDATORY: conventional commits, resumen corto, imperativo, una línea
  (`feat:`, `fix:`, `chore:`, `docs:`, `refactor:`, `test:`, `perf:`,
  `style:`, `build:`, `ci:`, `revert:`, scope opcional). <72 chars. Sin
  atribución de IA, sin trailers.
- Un cambio lógico por commit (sugerencia: `feat(backend): curp con digito
  verificador y correo del cliente` y `feat(backend): otp por telefono con
  whatsapp y mock`).
- Commitea SOLO tus archivos poseídos.
- BRANCH ISOLATION (obligatorio): commitea y pushea SOLO a tu rama de
  worktree — `git push origin <tu-rama>` — tras cada commit. Nunca a `main`
  ni a otra rama; nunca merge/rebase/fast-forward de ramas ajenas.

## Report back

- Archivos creados/cambiados, salida de `cargo test` (número de tests), el
  algoritmo del dígito verificador usado (fuente), y desviaciones del
  contrato si las hubo.