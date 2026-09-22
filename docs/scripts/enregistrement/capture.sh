#!/usr/bin/env bash
# La capture de l'écran d'administration, avec les identifiants du `.env` du projet courant.
set -euo pipefail
set -a
. ./.env
set +a
NODE_PATH="$ENREG_OUTILS/node_modules" node "$ENREG_ICI/capture-admin.cjs" \
  "$ENREG_TRAVAIL/admin.png" "$ADMIN_EMAIL" "$ADMIN_PASSWORD"
