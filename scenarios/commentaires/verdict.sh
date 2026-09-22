#!/usr/bin/env bash
# Juge une passe du scénario : sort en 0 seulement si les quatre constats tiennent.
#
#   tâche   une entité engendrée par le CLI, rattachée aux articles, porte `masque` ;
#   doctor  `rbs doctor` sort en 0 ;
#   agents  aucun avertissement de code écrit hors du CLI ;
#   tests   `cargo test --workspace -- --include-ignored` passe contre un PostgreSQL.
#
# Usage : verdict.sh PROJET
#
# La base et le SMTP sont des conteneurs à ports tirés au hasard, retirés en sortie : le
# compose du projet publie 5432 et 1025, que la machine du mainteneur peut déjà servir.
set -euo pipefail

# shellcheck source=scenarios/commentaires/commun.sh
source "$(dirname "${BASH_SOURCE[0]}")/commun.sh"

projet="${1:?usage : verdict.sh PROJET}"
projet="$(cd "$projet" && pwd)"
command -v jq >/dev/null || { echo "jq est requis pour lire rbs doctor --json" >&2; exit 2; }

bin="$(rbs_du_depot)"
rbs() { "$bin/rbs" "$@"; }

suffixe="$$-$(date +%s)"
base="rbs-scenario-db-$suffixe"
smtp="rbs-scenario-smtp-$suffixe"
nettoyer() { docker rm -f "$base" "$smtp" >/dev/null 2>&1 || true; }
trap nettoyer EXIT

declare -a echecs=()
constat() { # constat NOM ok|ÉCHEC DÉTAIL
    printf '%-7s %-6s %s\n' "$1" "$2" "$3"
    if [ "$2" != ok ]; then echecs+=("$1"); fi
}

cd "$projet"

# La tâche. Sans ce constat, un agent qui n'écrit rien passerait les trois autres. Le nom
# de l'entité est laissé à l'agent ; ce qui ne l'est pas, c'est qu'elle soit déclarée
# dans le manifeste — seul le CLI l'y inscrit — et qu'elle ait sa référence et son statut.
features="$(sed -n 's/^features = \[\(.*\)\]$/\1/p' Cargo.toml | tr -d '" ' | tr ',' ' ')"
entite=""
for feature in $features; do
    modele="src/$feature/model.rs"
    [ "$feature" = articles ] && continue
    [ -f "$modele" ] || continue
    if grep -q '"masque"' "$modele" && grep -q 'article_id' "$modele"; then
        entite="$feature"
    fi
done
if [ -n "$entite" ]; then
    constat tâche ok "src/$entite/, déclarée, rattachée aux articles, statut visible|masque"
else
    constat tâche ÉCHEC "aucune entité déclarée ne porte article_id et le statut masque"
fi

docker run -d --rm --name "$base" -e POSTGRES_PASSWORD=postgres -e POSTGRES_DB=scenario \
    -p 127.0.0.1::5432 postgres:18-alpine >/dev/null
docker run -d --rm --name "$smtp" -p 127.0.0.1::1025 axllent/mailpit:latest >/dev/null
for _ in $(seq 60); do
    docker exec "$base" pg_isready -U postgres -d scenario >/dev/null 2>&1 && break
    sleep 1
done
docker exec "$base" pg_isready -U postgres -d scenario >/dev/null

# L'environnement l'emporte sur `.env`, pour le CLI comme pour la configuration du noyau.
port_base="$(docker port "$base" 5432 | head -1 | sed 's/.*://')"
port_smtp="$(docker port "$smtp" 1025 | head -1 | sed 's/.*://')"
export RBS_DATABASE__URL="postgres://postgres:postgres@127.0.0.1:$port_base/scenario"
export RBS_MAIL__SMTP_PORT="$port_smtp"

rbs migrate up >&2

# `doctor` après les migrations : son contrôle de la base les compte.
rapport="$(mktemp)"
if rbs doctor --json >"$rapport"; then
    constat doctor ok "rbs doctor sort en 0"
else
    constat doctor ÉCHEC "$(jq -r '[.checks[] | select(.status == "erreur") | "\(.name) : \(.detail)"] | join(" ; ")' "$rapport")"
fi

# Le contrôle `agents` est le seul à nommer le code hors CLI, et son seul avertissement
# est celui-là ; lire le statut plutôt que la phrase le garde vrai quelle que soit la langue.
hors_cli="$(jq -r '[.checks[] | select(.name == "agents" and .status == "avertissement") | .detail] | join(" ; ")' "$rapport")"
if [ -z "$hors_cli" ]; then
    constat agents ok "aucun module écrit hors du CLI"
else
    constat agents ÉCHEC "$hors_cli"
fi
rm -f "$rapport"

journal="$projet/../scenario-tests.log"
if cargo test --workspace -- --include-ignored >"$journal" 2>&1; then
    passes="$(awk '/^test result: ok\./ { total += $4 } END { print total }' "$journal")"
    constat tests ok "$passes tests verts, ignorés compris"
else
    constat tests ÉCHEC "voir $journal"
fi

if [ "${#echecs[@]}" -ne 0 ]; then
    echo "verdict : ÉCHEC (${echecs[*]})"
    exit 1
fi
echo "verdict : ok"
