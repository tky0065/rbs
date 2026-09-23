# Sourcé, caché, par la première ligne de `terminal.tape` : le shell que filme VHS.
#
# VHS ne regarde pas le code de sortie de ce qu'il tape. Le piège ERR consigne donc chaque
# commande qui échoue dans `$ENREG_ECHECS`, que `regenere.sh` lit une fois l'enregistrement
# terminé : une commande en échec fait échouer la régénération, même quand l'image a l'air
# juste.

# shellcheck shell=bash
export PATH="$ENREG_BIN:$PATH"
export COMPOSE_PROJECT_NAME=rbs-enregistrement
# Les ports du compose engendré sont ceux d'une machine vierge (5432, 1025, 8025) : ceux
# de l'enregistrement les remplacent, pour ne jamais heurter une base déjà montée.
export COMPOSE_FILE="docker-compose.yml:$ENREG_ICI/compose.override.yml"
export GIT_AUTHOR_NAME=rbs GIT_AUTHOR_EMAIL=rbs@example.com
export GIT_COMMITTER_NAME=rbs GIT_COMMITTER_EMAIL=rbs@example.com

# `make dev &` place Vite en arrière-plan d'un terminal : ses raccourcis clavier y liraient
# l'entrée standard, et SIGTTIN arrêterait tout le job — le binaire avec. Vite ne les
# installe pas sous `CI`.
export CI=1

set -o pipefail
trap 'printf "%s\n" "$BASH_COMMAND" >> "$ENREG_ECHECS"' ERR

PS1='\[\e[38;5;244m\]\W\[\e[0m\] > '
cd "$ENREG_TRAVAIL/projets" || return 1
