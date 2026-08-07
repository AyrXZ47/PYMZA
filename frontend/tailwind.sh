#!/usr/bin/env bash
# Regenera assets/tailwind.css compilado. Obligatorio: el `dx` de nixpkgs NO
# compila tailwind automáticamente (solo copia el archivo ya compilado).
#   ./tailwind.sh            # compila una vez (tras cambiar clases)
#   ./tailwind.sh --watch    # compila y recompila en cada cambio
nix run nixpkgs#tailwindcss_4 -- -i ./tailwind.css -o ./assets/tailwind.css "$@"
