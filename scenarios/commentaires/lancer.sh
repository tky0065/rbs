#!/usr/bin/env bash
# Confie la tâche du scénario à Claude Code, dans le projet que `preparer.sh` a engendré.
#
# Usage : lancer.sh PROJET
#
# Jamais lancé par la CI : un agent n'est pas un test reproductible, et chaque passe coûte.
# L'agent tourne sans demander de permission — le projet est jetable, et une question
# restée sans réponse en mode `-p` arrêterait la passe.
set -euo pipefail

# shellcheck source=scenarios/commentaires/commun.sh
source "$(dirname "${BASH_SOURCE[0]}")/commun.sh"

projet="${1:?usage : lancer.sh PROJET}"
projet="$(cd "$projet" && pwd)"
prompt="$DEPOT/scenarios/commentaires/prompt.md"
journal="$projet/../scenario-agent.log"

bin="$(rbs_du_depot)"

{
    echo "rbs : $("$bin/rbs" --version)"
    echo "agent : $(claude --version)"
    echo "date : $(date -u +%Y-%m-%dT%H:%M:%SZ)"
} | tee "$journal"

# Le `rbs` du dépôt passe devant tout autre : c'est lui que l'`AGENTS.md` du projet
# désigne, et lui que le scénario éprouve.
cd "$projet"
PATH="$bin:$PATH" claude -p "$(cat "$prompt")" --dangerously-skip-permissions \
    | tee -a "$journal"
