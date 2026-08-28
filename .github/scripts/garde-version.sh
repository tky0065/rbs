#!/usr/bin/env bash
#
# Refuse un tag dont le numéro ne correspond pas aux versions publiées.
#
# crates.io ne reprend jamais une version : un `v0.4.0` posé sur un workspace resté en
# `0.1.0` publierait un `0.1.0` que plus rien ne pourrait corriger, et le tag mentirait
# pour toujours. Le refus doit donc précéder toute publication, dry-run compris.
#
#   usage : garde-version.sh <tag>

set -eu

if [ $# -ne 1 ]; then
  echo "garde-version.sh : un argument attendu, le nom du tag (reçu : $#)" >&2
  exit 2
fi

tag=$1

# La forme est vérifiée avant la comparaison : un `v0.4` qui se comparerait à `0.1.0`
# échouerait pour la bonne raison mais sur le mauvais diagnostic.
if ! printf '%s' "$tag" | grep -Eq '^v[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$'; then
  echo "garde-version.sh : tag « $tag » mal formé, attendu « vMAJEUR.MINEUR.CORRECTIF »" >&2
  exit 1
fi

attendue=${tag#v}
ecart=0

# Les deux crates sont interrogées séparément plutôt qu'à travers `[workspace.package]` :
# elles en héritent aujourd'hui, mais celle qui en sortirait rendrait silencieusement
# fausse une garde qui ne lit que le workspace.
#
# `cargo pkgid` plutôt qu'un grep sur le manifeste : la version héritée n'y figure pas.
# Sa sortie est `path+file:///…/rbs-core#0.1.0`, ou `…#nom@version` quand le répertoire
# ne porte pas le nom du paquet — les deux séparateurs se coupent d'un coup.
for paquet in rbs-core rbs-cli; do
  publiee=$(cargo pkgid -p "$paquet" | sed 's/.*[@#]//')
  if [ "$publiee" != "$attendue" ]; then
    echo "garde-version.sh : le tag « $tag » annonce $attendue, or $paquet est en $publiee" >&2
    ecart=1
  fi
done

if [ "$ecart" -ne 0 ]; then
  echo "garde-version.sh : publication refusée, aucune crate n'a été envoyée" >&2
  exit 1
fi

echo "garde-version.sh : tag $tag concordant, rbs-core et rbs-cli sont en $attendue"
