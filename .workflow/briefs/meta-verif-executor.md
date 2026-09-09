# Brief: Micro-tarea · Executor único (meta-verif)

> Micro-tarea fuera de ola: inyectar la metaetiqueta de verificación de dominio
> de Meta en el HTML shell del frontend. Un archivo, un commit. El executor
> nunca toca un archivo que no posee, ni "obviamente". Desviaciones → planner
> vía decision log en `.workflow/plan.md`.

## Task

Crear `frontend/index.html` como template del HTML shell de Dioxus 0.7.9 con la
etiqueta de verificación de Meta dentro de `<head>`:

```html
<meta name="facebook-domain-verification" content="2evl3zta04uil86lc7sqfo9begdn1g" />
```

Hoy no existe ese archivo: dx lo GENERA solo desde `Dioxus.toml`. Cuando existe
un `index.html` en la raíz del crate, dx lo usa como template verbatim y
**inyecta automáticamente** el `<link rel="preload">` y el `<script type="module">`
del WASM — NO los copies manualmente (quedarían duplicados o confundirían al
inyector).

El template debe replicar el shell generado (verificado en vivo en producción):

```html
<!DOCTYPE html>
<html>
    <head>
        <title>PYMZA — Crédito con cobranza respaldada</title>
        <meta content="text/html;charset=utf-8" http-equiv="Content-Type">
        <meta name="viewport" content="width=device-width, initial-scale=1">
        <meta charset="UTF-8">
        <meta name="facebook-domain-verification" content="2evl3zta04uil86lc7sqfo9begdn1g" />
    </head>
    <body>
        <div id="main"></div>
    </body>
</html>
```

Requisitos duros: `<div id="main"></div>` EXACTO (punto de montaje de Dioxus),
título idéntico a `Dioxus.toml` (`PYMZA — Crédito con cobranza respaldada`),
metas de charset/viewport presentes.

**Fallback documentado** (solo si `dx build` falla o el shell de salida pierde
el título/el div#main): en su lugar, regresa el archivo creado y agrega UNA
línea `RUN` en `Dockerfile.frontend` después del `RUN dx build ...` que inserte
la metaetiqueta con `sed` en
`/app/target/dx/frontend/release/web/public/index.html`, y commitea eso. En
cualquier caso reporta cuál de las dos vías usaste y por qué.

## Definition of done

- `frontend/index.html` creado con el contenido de arriba (o fallback aplicado
  en `Dockerfile.frontend`).
- `dx build --release` termina en exit 0 SIN duplicar inyecciones (el HTML de
  salida tiene exactamente UN tag `<script type="module"` y UN
  `rel="preload"` del bundle).
- El index.html de salida contiene la etiqueta `facebook-domain-verification`
  exactamente 1 vez, conserva el título `PYMZA — Crédito con cobranza
  respaldada` y `div id="main"`.
- No hay cambios en ningún otro archivo (el CSS/Tailwind NO se regenera: el
  `assets/tailwind.css` compilado ya está commiteado y el shell no lo afecta).
- El verify command pasa.

## Files you own

- `frontend/index.html` (nuevo)

## Files forbidden

- `frontend/Dioxus.toml`, `frontend/src/**`, `frontend/assets/**`,
  `frontend/tailwind.css`, `frontend/tailwind.sh`
- `Dockerfile.frontend` (excepto en el camino de fallback documentado arriba;
  si lo tocas, reporta la desviación)
- Todo lo demás del repo

## Read first

- `Dockerfile.frontend` (líneas 15–35: qué copia dx build y dónde)
- `frontend/Dioxus.toml` (el título del shell)
- `backend/AGENTS.md` no aplica; `frontend/AGENTS.md` solo si editas código Rust (no lo harás)

## Verify command

```bash
cd frontend && dx build --release --debug-symbols=false \
  && grep -c 'facebook-domain-verification' target/dx/frontend/release/web/public/index.html \
  && grep -c 'id="main"' target/dx/frontend/release/web/public/index.html \
  && grep -c 'PYMZA — Crédito con cobranza respaldada' target/dx/frontend/release/web/public/index.html
# Esperado: 1 \n 1 \n 1 (la primera línea es la etiqueta de Meta)
```

## Commit

- MANDATORY: conventional commits, short summary, imperative, one line
  (`chore:` en este caso). Under ~72 chars. No AI attribution, no trailers.
- Un commit: `chore: add meta domain verification to html shell`
- Commit ONLY tu archivo.
- BRANCH ISOLATION (mandatory): commit y push SOLO a tu rama de worktree
  (`git push origin meta-verif-executor`). Nunca a `main` ni a otra rama;
  nunca merge, rebase ni fast-forward ajeno.

## Report back

- Qué vía usaste (template nativo vs fallback sed) y por qué.
- Salida completa del verify command.
- `git log --oneline -1` de tu rama.
- Cualquier sorpresa (p. ej. dx quejándose del template, inyecciones duplicadas
  en el HTML de salida).
