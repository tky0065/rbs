#!/usr/bin/env bash
# Sourcé par les trois scripts du scénario : le dépôt, et le `rbs` qu'il compile.
#
# Le scénario éprouve le CLI du dépôt, jamais celui qu'une machine a installé : sur un
# poste où `rbs` désigne l'outil de typage de Ruby, un `rbs` pris dans le PATH ferait
# échouer la passe pour une raison étrangère à rbs.

DEPOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

# Compile le binaire du dépôt et rend le répertoire qui le contient.
rbs_du_depot() {
    cargo build --quiet --manifest-path "$DEPOT/Cargo.toml" -p rbs-cli --bin rbs >&2
    local cible
    cible="$(cargo metadata --manifest-path "$DEPOT/Cargo.toml" --format-version 1 --no-deps \
        | sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p')"
    printf '%s/debug\n' "$cible"
}
