# F10 — CI complète : Linux, macOS, Windows

## Décision

Deux jobs, pas une matrice unique sur le job existant. Le job actuel mêle des contrôles
de *plateforme* (`test`, `clippy`) et des contrôles de *dépôt* (`fmt`, `clippy` des
exemples, `publish --dry-run`). Tripler les seconds coûte des minutes de runner sans rien
démontrer.

Les runners GitHub macOS n'exposent aucun démon Docker et `windows-latest` ne fait tourner
que des conteneurs Windows. Les tests `#[ignore]` de `rbs-cli` démarrent un PostgreSQL via
testcontainers : ils restent sur Linux. Ce que macOS et Windows doivent prouver, c'est que
le code compile et que les chemins se manipulent correctement — pas que Docker fonctionne.

## Étapes

1. `.gitattributes` avec `* text=auto eol=lf`. Sans lui, `core.autocrlf=true` (défaut de
   Git for Windows sur les runners) fait extraire les templates en CRLF ; `include_dir!`
   embarque ces CRLF et les tests qui comparent la sortie générée à des littéraux `\n`
   échouent pour un motif étranger à rbs.
2. `ci.yml` : renommer le job existant en `linux`, inchangé quant à ses étapes.
3. Ajouter le job `portabilite`, matrice `[macos-latest, windows-latest]`, `fail-fast:
   false` — un échec Windows ne doit pas masquer le résultat macOS. Étapes : `clippy
   --workspace --all-targets -- -D warnings` puis `cargo test --workspace`, sans
   `--ignored`.
4. `fmt` reste sur Linux : contrôle de dépôt, et l'exposer aux fins de ligne Windows
   ouvrirait un faux échec.

## Preuve

Le critère est « les trois plateformes passent au vert ». Sous la contrainte « local
seulement » retenue pour ce lot :

- Linux : `act` sur le job `linux`.
- macOS : natif, la machine de développement est Darwin arm64 — les commandes exactes du
  job `portabilite`.
- Windows : **non prouvable ici**. Ni runner ni émulation.

Deux sur trois ⇒ la case reste `- [ ]` avec une annotation `PARTIEL` nommant ce qui
manque : un run réel sur `windows-latest`.
