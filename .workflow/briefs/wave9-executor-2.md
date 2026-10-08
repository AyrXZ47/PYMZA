# Brief: Wave 9 · Executor 2 (frontend + marca)

> Lee `.workflow/plan.md` §"Ola 9 (actual)" y el aviso
> `aviso-privacidad-integral-2026-10.md` antes de escribir. Este brief resume; el
> plan manda.

## Task

Cumplimiento y marca en el frontend:

1. **Páginas legales de texto** (no PDF, sin dep de markdown): `Aviso de
   Privacidad` (contenido de `aviso-privacidad-integral-2026-10.md`) y `Términos
   de Servicio` (`docs/legal/terminos-de-servicio.md` de executor-1). Accesibles
   desde la landing (footer) y desde el login/registro. Si el build de Dioxus lo
   permite, usa una sola fuente de verdad (p. ej. `include_str!` del `.md` y render
   por líneas); si no, texto en un componente y deja constancia.
2. **Checkbox de aceptación en el registro** (`components/registro.rs`): casilla
   NO premarcada "He leído y acepto el Aviso de Privacidad y los Términos de
   Servicio", con enlaces a ambas páginas. El botón de crear cuenta queda
   deshabilitado hasta marcarla. Al enviar, manda `acepta_aviso: true` y
   `aviso_version` (const, "2026-10-07") al `POST /api/empresas` (contrato de
   executor-1). Muestra el error del backend si falta.
3. **Rebrand PYMZA → PIGNUS** en todo lo visible: landing, login, registro,
   sidebar, `<title>`/meta, mensajes, textos de ayuda. Cero "PYMZA" visible en la
   UI (`rg -i pymza frontend/` sin coincidencias de cara al usuario). Revisa
   `frontend/index.html` si el rebrand del `<head>` lo necesita.
4. **Próximos cobros (petición de V)**: vuelve la tarjeta al dashboard consumiendo
   `proximos_cobros` + `monto_proximos_cobros` por ventana, con selector de 15/30
   días (ventana móvil, default 30). Nada de "todo el futuro".
5. **Marca/licencia**: crea `LICENSE-PROPRIETARY.md` ("Copyright © 2026 Arnold
   Yovick Rios Zaldivar. Todos los derechos reservados…") y una nota breve en
   `README.md` (producto PIGNUS; repo privado; vitrina pública separada). Actualiza
   `AGENTS.md` para que el producto se llame PIGNUS (sin cambiar la mecánica del
   workflow).
6. Regenera `frontend/assets/tailwind.css` con `./tailwind.sh` si agregas clases.

## Definition of done

- Aviso y ToS alcanzables desde landing y registro; se leen completos y sin PDF.
- Sin la casilla marcada, no se puede crear cuenta; con ella, el backend persiste
  la evidencia (verificado con executor-1).
- `rg -i "pymza" frontend/src frontend/index.html` → 0 coincidencias visibles.
- La tarjeta "Próximos cobros" muestra conteo/monto de la ventana y cambia al
  alternar 15/30 días.
- `LICENSE-PROPRIETARY.md` creado y README/AGENTS rebrandeados.
- Verify command passes.

## Files you own

- `frontend/src/**`, `frontend/index.html`, `frontend/tailwind.css`,
  `frontend/assets/tailwind.css`
- `README.md`, `AGENTS.md`
- `aviso-privacidad-integral-2026-10.md` (ajustes de render si hacen falta)
- `LICENSE-PROPRIETARY.md` (NUEVO)

## Files forbidden

- TODO `backend/**` y `docs/**` (executor-1; el ToS lo escribe él).
- `.workflow/**`, `skills/**`, `docs/DEPLOY.md`, `Dockerfile.*` (si el `<head>` lo
  exige, repórtalo), `docker-compose.yml`, `.env*`, `PIGNUS.md` (symlink de la
  nota de V; no tocar).

## Read first

- `.workflow/plan.md` §"Ola 9 (actual)".
- `aviso-privacidad-integral-2026-10.md` (contenido del aviso).
- `frontend/src/components/registro.rs` (alta de empresa) y `api.rs` (contratos).
- `frontend/src/components/dashboard.rs` (KPI cards de la ola 8) y `charts.rs`.
- `frontend/src/components/landing.rs` (footer/enlaces).
- `frontend/AGENTS.md` (API Dioxus 0.7).

## Verify command

```bash
cd frontend && cargo check --target wasm32-unknown-unknown && cargo test && ./tailwind.sh
# y el humo visual del registro con checkbox (navegador, owner V)
```

## Commit

- Conventional commits, una línea, sin atribución de IA. Commits separados
  (`feat(frontend): paginas de aviso y terminos` / `feat(frontend): checkbox de
  aceptacion` / `feat(frontend): tarjeta proximos cobros` / `chore: rebrand
  PIGNUS` / `chore: licencia propietaria`).
- BRANCH ISOLATION: `git push origin wave9-executor-2` tras cada commit.

## Report back

- Archivos, salida del verify, decisiones de render del aviso, y cualquier
  desviación o pregunta (p. ej. si el `<title>`/meta exige tocar Dockerfile).
