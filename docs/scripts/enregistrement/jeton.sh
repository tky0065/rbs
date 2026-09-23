#!/usr/bin/env bash
# Le jeton d'accès du compte que `rbs seed` a posé, lu depuis le projet courant.
set -euo pipefail
set -a
. ./.env
set +a
curl -sS --fail-with-body localhost:8080/auth/login \
  -H 'content-type: application/json' \
  -d "{\"email\":\"$ADMIN_EMAIL\",\"password\":\"$ADMIN_PASSWORD\"}" | jq -er .access_token
