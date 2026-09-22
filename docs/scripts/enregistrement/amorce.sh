#!/usr/bin/env bash
# Ce que l'enregistrement coupe au montage, entre `rbs generate crud` et `make dev` : la
# base, les migrations, le compte d'administration, `npm install`, le client typé et la
# compilation. Plusieurs minutes que personne ne regarderait, lancées depuis le projet.
set -euo pipefail

git add -A
git commit -qm "rbs generate crud tickets"

perl -pi -e 's/localhost:5432/localhost:55471/' .env
echo 'RBS_MAIL__SMTP_PORT=55472' >> .env

docker compose up -d --wait
rbs migrate up
rbs seed
(cd frontend && npm install --no-audit --no-fund)
rbs generate client --lang ts
cargo build
