#!/usr/bin/env bash
# Engendre le projet de départ du scénario : `auth`, `frontend-admin` et une ressource
# `articles`, par le CLI et le `rbs-core` du dépôt.
#
# Usage : preparer.sh [RÉPERTOIRE]   — le projet est créé sous RÉPERTOIRE/blog ;
#                                      à défaut, dans un répertoire temporaire neuf.
set -euo pipefail

# shellcheck source=scenarios/commentaires/commun.sh
source "$(dirname "${BASH_SOURCE[0]}")/commun.sh"

parent="${1:-$(mktemp -d "${TMPDIR:-/tmp}/rbs-scenario.XXXXXX")}"
mkdir -p "$parent"
parent="$(cd "$parent" && pwd)"
projet="$parent/blog"

if [ -e "$projet" ]; then
    echo "$projet existe déjà : le scénario part d'un projet fraîchement engendré" >&2
    exit 1
fi

bin="$(rbs_du_depot)"

(
    cd "$parent"
    "$bin/rbs" new blog --with auth,frontend-admin --lang fr --yes \
        --core-path "$DEPOT/crates/rbs-core"
)

(
    cd "$projet"
    "$bin/rbs" generate crud articles --fields "title:string,body:text"

    # Le point de départ est commité : `git diff` montre ensuite tout ce que l'agent a
    # écrit, et la garde Git du CLI le trouve propre, comme sur un vrai dépôt.
    git add -A
    git -c user.name="scénario rbs" -c user.email="scenario@rbs.invalid" \
        commit --quiet -m "projet de départ du scénario"
)

echo
echo "projet prêt : $projet"
