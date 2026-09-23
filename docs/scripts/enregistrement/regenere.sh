#!/usr/bin/env bash
# Régénère l'enregistrement en tête du README : ses deux parts, d'une seule commande.
#
#     docs/scripts/enregistrement/regenere.sh
#
# Le terminal est filmé par VHS depuis `terminal.tape` ; l'écran d'administration est
# capturé par Playwright sur le même projet, pendant que `make dev` tourne encore. Le
# projet est engendré par le `rbs` du dépôt, sur le `rbs-core` du dépôt, dans un
# répertoire temporaire que le script efface en sortant, base comprise.
#
# Le script échoue si une commande du tape rend un code non nul, si une attente n'aboutit
# pas, ou si l'une des deux images manque : rien n'est alors copié dans `docs/static/`.
#
# Prérequis : vhs (qui tire ttyd et ffmpeg), Docker démarré, jq, node et npm, cargo. Les
# ports 8080 et 5173 (le binaire et Vite), 55471 et 55472 (la base et le SMTP) doivent
# être libres.
set -euo pipefail

ICI=$(cd "$(dirname "$0")" && pwd)
DEPOT=$(cd "$ICI/../../.." && pwd)
DEST="$DEPOT/docs/static/img/enregistrement"
OUTILS="$DEPOT/target/enregistrement-outils"
PLAYWRIGHT=1.63.0

echoue() {
  printf 'erreur : %s\n' "$*" >&2
  exit 1
}

for outil in vhs ttyd ffmpeg docker jq node npm make cargo curl; do
  command -v "$outil" > /dev/null || echoue "$outil introuvable"
done
docker info > /dev/null 2>&1 || echoue "Docker ne répond pas"
for port in 8080 5173 55471 55472; do
  if (exec 3<> "/dev/tcp/127.0.0.1/$port") 2> /dev/null; then
    echoue "le port $port est déjà pris"
  fi
done

TRAVAIL=$(mktemp -d "${TMPDIR:-/tmp}/rbs-enregistrement.XXXXXX")
TRAVAIL=$(cd "$TRAVAIL" && pwd -P)

# Le binaire se lance par un chemin relatif (`target/debug/tickets`) : on le retrouve par
# son répertoire courant, et non par sa ligne de commande. Un processus arrêté (état T) ne
# reçoit TERM qu'une fois relancé, d'où le CONT.
nettoie() {
  local pid
  for pid in $(lsof -t -iTCP:8080 -iTCP:5173 -sTCP:LISTEN 2> /dev/null); do
    if lsof -a -p "$pid" -d cwd -Fn 2> /dev/null | grep -q "^n$TRAVAIL"; then
      kill -TERM "$pid" 2> /dev/null || true
      kill -CONT "$pid" 2> /dev/null || true
    fi
  done
  pkill -TERM -f "$TRAVAIL" 2> /dev/null || true
  pkill -CONT -f "$TRAVAIL" 2> /dev/null || true
  docker compose -p rbs-enregistrement down -v > /dev/null 2>&1 || true
  rm -rf "$TRAVAIL"
}
trap nettoie EXIT

echo "→ rbs, compilé depuis le dépôt"
cargo build --quiet --manifest-path "$DEPOT/Cargo.toml" -p rbs-cli --bin rbs
# La cible est celle que cargo annonce, et non `target/` : un `CARGO_TARGET_DIR` posé
# ferait filmer un `rbs` absent ou périmé.
CIBLE="$(cargo metadata --manifest-path "$DEPOT/Cargo.toml" --format-version 1 --no-deps \
  | sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p')"
BIN="$CIBLE/debug"
VERSION=$("$BIN/rbs" --version)

echo "→ Playwright $PLAYWRIGHT et son Chromium"
if [ "$(node -p "require('$OUTILS/node_modules/playwright/package.json').version" 2> /dev/null)" != "$PLAYWRIGHT" ]; then
  npm install --silent --no-audit --no-fund --prefix "$OUTILS" "playwright@$PLAYWRIGHT"
fi
"$OUTILS/node_modules/.bin/playwright" install chromium > /dev/null

# Le projet enregistré dépend du `rbs-core` du dépôt sans que la commande filmée le dise :
# le `[patch]` vit dans la configuration Cargo du répertoire qui le contient.
mkdir -p "$TRAVAIL/.cargo" "$TRAVAIL/projets"
printf '[patch.crates-io]\nrbs-core = { path = "%s/crates/rbs-core" }\n' "$DEPOT" \
  > "$TRAVAIL/.cargo/config.toml"

export ENREG_ICI="$ICI" ENREG_BIN="$BIN" ENREG_TRAVAIL="$TRAVAIL" ENREG_OUTILS="$OUTILS"
export ENREG_ECHECS="$TRAVAIL/echecs.txt"

echo "→ enregistrement de $VERSION (plusieurs minutes : la compilation est coupée au montage)"
# Une commande en échec ne fait pas toujours échouer VHS, et un VHS en échec (une attente
# qui n'aboutit pas) vient souvent d'une commande en échec plus haut : les deux se disent.
VHS_OK=1
(cd "$TRAVAIL" && vhs "$ICI/terminal.tape") || VHS_OK=0
if [ "$VHS_OK" = 0 ] || [ -s "$ENREG_ECHECS" ]; then
  if [ -s "$ENREG_ECHECS" ]; then
    printf "commande en échec pendant l'enregistrement :\n" >&2
    sed 's/^/  /' "$ENREG_ECHECS" >&2
  fi
  tail -n 20 "$TRAVAIL/amorce.log" "$TRAVAIL/capture.log" 2> /dev/null || true
  [ "$VHS_OK" = 1 ] || echoue "VHS n'a pas mené l'enregistrement à son terme"
  exit 1
fi
[ -s "$TRAVAIL/images/frame-text-00001.png" ] || echoue "VHS n'a produit aucune image"

# La cadence est celle de `Set Framerate` dans le tape, la marge et son fond ceux de
# `Set Padding` et du thème, que VHS n'applique qu'à l'assemblage. Une palette par GIF,
# calculée sur les seules différences d'une image à l'autre : un terminal change peu, et
# le poids suit.
echo "→ assemblage du GIF"
(
  cd "$TRAVAIL/images"
  ffmpeg -loglevel error -y \
    -framerate 12 -i frame-text-%05d.png \
    -framerate 12 -i frame-cursor-%05d.png \
    -filter_complex '[0][1]overlay,pad=iw+48:ih+48:24:24:color=0x1e1e2e,split[a][b];[a]palettegen=stats_mode=diff[p];[b][p]paletteuse=dither=none:diff_mode=rectangle' \
    "$TRAVAIL/terminal.gif"
)
[ -s "$TRAVAIL/terminal.gif" ] || echoue "terminal.gif n'a pas été produit"
[ -s "$TRAVAIL/admin.png" ] || echoue "admin.png n'a pas été produit"

mkdir -p "$DEST"
cp "$TRAVAIL/terminal.gif" "$TRAVAIL/admin.png" "$DEST/"
echo "✓ $VERSION → docs/static/img/enregistrement/ (terminal.gif, admin.png)"
