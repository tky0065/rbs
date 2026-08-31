# Hygiène de la CI et du workspace — plan d'implémentation

> **Pour l'exécutant :** SOUS-SKILL REQUIS : `superpowers:executing-plans`. Les étapes se
> suivent par cases à cocher.

**Objectif :** aligner le harnais de test sur la version de PostgreSQL réellement livrée,
faire dire à la CI la vérité sur les advisories et sur la MSRV, et retirer du workspace deux
dépendances que personne ne consomme.

**Architecture :** quatre changements indépendants, sans code applicatif. Trois touchent des
constantes ou des manifestes, le quatrième ajoute deux jobs à `.github/workflows/ci.yml`. La
règle qui les relie : ce que le dépôt affirme doit être ce qu'une commande exécutable
constate.

**Tech Stack :** Cargo (workspace, `[workspace.dependencies]`, `rust-version`),
`testcontainers`, `cargo-audit`, GitHub Actions.

**Spec :** pas de spec dédiée — le lot exécute quatre entrées du backlog `IMPROVE.md` (42,
43, 44, 50), section P2, toutes classées *Easy*.

## Contraintes globales

- Aucune modification de `IMPROVE.md` : le suivi appartient à l'orchestrateur.
- Les templates de `crates/rbs-cli/templates/` décrivent le manifeste d'un *projet engendré*
  et sont hors périmètre : `tower-http` et `utoipa-swagger-ui` y sont légitimes.
- `postgres:18-alpine` reste la version livrée partout ; le harnais s'aligne sur elle, jamais
  l'inverse.
- MSRV réelle mesurée : **1.94**. `cargo +1.85 check --workspace --all-targets --all-features`
  refuse de résoudre — `sea-orm@2.0.2 requires rustc 1.94.0` est le plancher le plus haut de
  l'arbre.
- Documentation bilingue : toute page anglaise modifiée l'est aussi en français, dans le même
  commit.
- Commits : Conventional Commits, sujet en français à l'impératif, corps avec le *pourquoi* et
  un intertitre `Vérifications :`.

---

### Tâche 1 : aligner le harnais PostgreSQL sur 18

**Fichiers :**
- Modifier : `crates/rbs-cli/tests/common/mod.rs:160`
- Modifier : `crates/rbs-cli/tests/integration_crud.rs:19`
- Modifier : `crates/rbs-cli/tests/integration_auth.rs:28`
- Modifier : `crates/rbs-cli/src/generate/bench.rs:41`
- Modifier : `docs/docs/guides/testing.md:68`
- Modifier : `docs/i18n/fr/docusaurus-plugin-content-docs/current/guides/testing.md:71`

**Interfaces :**
- Produit : `common::IMAGE == ("postgres", "18")`, consommé par les tests d'intégration qui
  passent par `common`.

- [ ] **Étape 1 : recenser les points d'ancrage**

```bash
grep -rn '"postgres", "17"' crates/
```

Attendu : quatre lignes — `tests/common/mod.rs`, `tests/integration_crud.rs`,
`tests/integration_auth.rs`, `src/generate/bench.rs`. Le backlog n'en nommait que deux.

- [ ] **Étape 2 : porter les quatre à 18**

```bash
grep -rl '"postgres", "17"' crates/ | xargs sed -i '' 's/"postgres", "17"/"postgres", "18"/'
grep -rn '"postgres", "1[78]"' crates/
```

Attendu : quatre lignes, toutes en `"18"`.

- [ ] **Étape 3 : corriger les deux pages qui annonçaient 17**

`docs/docs/guides/testing.md` : « a PostgreSQL 17 container » → « a PostgreSQL 18 container ».
`docs/i18n/.../guides/testing.md` : « PostgreSQL 17 avec `testcontainers` » → « PostgreSQL 18
avec `testcontainers` ». Ne pas toucher aux phrases voisines sur le plancher 14, qui restent
vraies.

- [ ] **Étape 4 : vérifier qu'aucune autre version ne traîne**

```bash
grep -rn 'postgres:1[0-9]\|"postgres", "1[0-9]"' crates/ examples/ docs/docs docs/i18n .github/
```

Attendu : uniquement `18` et `18-alpine`.

- [ ] **Étape 5 : compiler et lancer la suite non-Docker**

```bash
cargo test --workspace
```

Attendu : `ok` sur tous les binaires de test.

- [ ] **Étape 6 : lancer les tests Docker**

```bash
cargo test --workspace --no-fail-fast -- --ignored
```

`--no-fail-fast` est obligatoire : sans lui la suite s'arrête au premier binaire et masque les
échecs suivants. Attendu : PostgreSQL 18 démarre et tous les tests `--ignored` passent.

- [ ] **Étape 7 : commit**

```bash
git add crates/rbs-cli docs/docs/guides/testing.md \
  docs/i18n/fr/docusaurus-plugin-content-docs/current/guides/testing.md
git commit -m "test(cli): aligne le harnais PostgreSQL sur la version livrée"
```

---

### Tâche 2 : retirer les deux dépendances de workspace inutilisées

**Fichiers :**
- Modifier : `Cargo.toml:43` (`tower-http`), `Cargo.toml:48` (`utoipa-swagger-ui`)

- [ ] **Étape 1 : prouver qu'elles ne sont consommées nulle part**

```bash
grep -rn 'tower_http\|utoipa_swagger_ui' crates/*/src
grep -n 'tower-http\|utoipa-swagger-ui' crates/*/Cargo.toml
grep -n 'name = "tower-http"\|name = "utoipa-swagger-ui"' Cargo.lock
```

Attendu : les trois commandes sortent à vide. Ne pas confondre avec les occurrences de
`crates/rbs-cli/src/templates.rs` et `crates/rbs-cli/src/add/mod.rs`, qui sont des chaînes
décrivant le manifeste d'un projet engendré.

- [ ] **Étape 2 : supprimer les deux lignes**

Retirer de `[workspace.dependencies]` :

```toml
tower-http = "0.7.0"
utoipa-swagger-ui = "9.0.2"
```

`tower = "0.5.3"` reste : `rbs-core` l'emploie en `[dev-dependencies]`.

- [ ] **Étape 3 : vérifier que le lock ne bouge pas**

```bash
md5 Cargo.lock && cargo metadata --format-version 1 >/dev/null && md5 Cargo.lock
```

Attendu : deux empreintes identiques — ces deux dépendances n'entraient dans aucun graphe de
résolution.

- [ ] **Étape 4 : compiler**

```bash
cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings
```

Attendu : vert des deux côtés.

- [ ] **Étape 5 : commit**

```bash
git add Cargo.toml
git commit -m "build: retire deux dépendances de workspace que personne ne consomme"
```

---

### Tâche 3 : porter la MSRV déclarée à ce que le code exige

**Fichiers :**
- Modifier : `Cargo.toml:12`
- Modifier : `README.md:26`, `README.fr.md:27`
- Modifier : `CONTRIBUTING.md:22`, `CONTRIBUTING.fr.md:23`
- Modifier : `crates/rbs-core/README.md:60`, `crates/rbs-cli/README.md:22`
- Modifier : `CHANGELOG.md` (section `## [Unreleased]` → `### Changed`)
- Modifier : `CHANGELOG.fr.md` (section `## [Non publié]` → `### Modifié`)

**Interfaces :**
- Produit : `rust-version = "1.94"` dans `[workspace.package]`, dont les deux crates héritent
  par `rust-version.workspace = true`. La tâche 4 épingle un job de CI sur cette valeur.

- [ ] **Étape 1 : mesurer le plancher réel**

```bash
rustup toolchain install 1.85 --profile minimal
cargo +1.85 check --workspace --all-targets --all-features
```

Attendu : `error: rustc 1.85.1 is not supported by the following packages:` suivi d'une liste
dont le maximum est `sea-orm@2.0.2 requires rustc 1.94.0`. La MSRV déclarée est donc fausse.

- [ ] **Étape 2 : confirmer que 1.94 suffit**

```bash
rustup toolchain install 1.94 --profile minimal
cargo +1.94 check --workspace --all-targets --all-features
```

Attendu : `Finished \`dev\` profile`.

- [ ] **Étape 3 : porter le manifeste**

Dans `Cargo.toml`, `rust-version = "1.85"` → `rust-version = "1.94"`.

- [ ] **Étape 4 : porter les six lignes de documentation**

Six phrases affirment « Rust 1.85 » et deviendraient fausses. Les porter à 1.94 :
`README.md`, `README.fr.md`, `CONTRIBUTING.md`, `CONTRIBUTING.fr.md`,
`crates/rbs-core/README.md`, `crates/rbs-cli/README.md`. Ne pas toucher au `CHANGELOG` de la
version 1.1.0 déjà parue : il consigne ce qui était vrai à sa date.

- [ ] **Étape 5 : consigner le changement dans le journal**

Ajouter sous `### Changed` de `## [Unreleased]` (`CHANGELOG.md`) et sous `### Modifié` de
`## [Non publié]` (`CHANGELOG.fr.md`) une entrée disant que le plancher passe de 1.85 à 1.94,
et pourquoi : `sea-orm 2.0.2` l'exige, et 1.85 ne résolvait déjà plus.

- [ ] **Étape 6 : vérifier qu'aucune mention de 1.85 ne subsiste hors historique**

```bash
grep -rn '1\.85' README.md README.fr.md CONTRIBUTING.md CONTRIBUTING.fr.md \
  Cargo.toml crates/*/README.md
```

Attendu : sortie vide.

- [ ] **Étape 7 : commit**

```bash
git add Cargo.toml README.md README.fr.md CONTRIBUTING.md CONTRIBUTING.fr.md \
  crates/rbs-core/README.md crates/rbs-cli/README.md CHANGELOG.md CHANGELOG.fr.md
git commit -m "build: porte la MSRV déclarée au plancher que les dépendances imposent"
```

---

### Tâche 4 : faire dire la vérité à la CI sur les advisories et sur la MSRV

**Fichiers :**
- Créer : `.cargo/audit.toml`
- Modifier : `.github/workflows/ci.yml` (deux jobs ajoutés après `linux`)

**Interfaces :**
- Consomme : `rust-version = "1.94"` posé par la tâche 3 — le job `msrv` épingle la même
  valeur, et une divergence entre les deux est précisément ce que le job détecte.

- [ ] **Étape 1 : partir de l'état réel**

```bash
cargo install cargo-audit --locked   # si absent
cargo audit
```

Attendu : `error: 2 vulnerabilities found!` — `RUSTSEC-2026-0235` (`rkyv` 0.7.46) et
`RUSTSEC-2023-0071` (`rsa` 0.9.10), plus un avertissement toléré sur `proc-macro-error2`.

- [ ] **Étape 2 : établir si `rkyv` se purge**

```bash
cargo update -p rkyv
cargo tree --workspace --all-features -i rkyv
```

Attendu : `Locking 0 packages` — `rust_decimal` 1.42.1, dernière version parue, déclare
`rkyv` en dépendance optionnelle `^0.7.46`, quand l'advisory demande `>= 0.8.17`. Et
`cargo tree -i` rend `nothing to print` : la feature n'est jamais activée, l'entrée du lock
est orpheline. La purge annoncée par le backlog n'est pas possible.

- [ ] **Étape 3 : écrire `.cargo/audit.toml`**

```toml
# `cargo audit` lit le lock, qui ignore les features : il voit des dépendances optionnelles
# que rien n'active. Les deux entrées ci-dessous sont là pour cette raison ou faute de
# correctif amont, et chacune se lève par une commande, pas par un jugement.

[advisories]
ignore = [
    # rsa 0.9.10, Marvin Attack. Tiré par `jsonwebtoken`, sans correctif amont : l'advisory
    # dit « No fixed upgrade is available! ». À lever le jour où `rsa` publie un correctif —
    # `cargo audit` le signalera de lui-même, l'entrée devenant une règle sans effet.
    "RUSTSEC-2023-0071",

    # rkyv 0.7.46. Dépendance optionnelle de `rust_decimal`, jamais compilée :
    # `cargo tree --workspace --all-features -i rkyv` rend « nothing to print ». Le correctif
    # demande rkyv >= 0.8.17, que `rust_decimal` 1.42.1 ne peut pas prendre — il épingle
    # `^0.7.46`. À lever quand `rust_decimal` passera à rkyv 0.8.
    "RUSTSEC-2026-0235",
]
```

- [ ] **Étape 4 : vérifier que l'audit passe au vert**

```bash
cargo audit; echo "code de sortie : $?"
```

Attendu : `code de sortie : 0`, l'avertissement `proc-macro-error2` restant affiché sans faire
échouer — `cargo audit` ne rend non nul que sur une vulnérabilité.

- [ ] **Étape 5 : ajouter les deux jobs à la CI**

Après le job `linux`, dans `.github/workflows/ci.yml` :

```yaml
  # Un audit ne compile rien : le tenir hors du job `linux` le rend en une poignée de
  # secondes, et son verdict ne se perd pas derrière vingt minutes de compilation.
  #
  # `cargo audit` nu plutôt qu'une action qui l'enveloppe : c'est la commande que le
  # développeur lance chez lui, et elle lit le même `.cargo/audit.toml`. Un écart entre les
  # deux verdicts serait exactement le genre de mensonge que ce job existe pour interdire.
  audit:
    name: cargo audit
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7

      - uses: dtolnay/rust-toolchain@stable

      - uses: taiki-e/install-action@v2
        with:
          tool: cargo-audit

      - name: cargo audit
        run: cargo audit

  # `rust-version` est une promesse faite aux utilisateurs, et rien ne la tenait : tous les
  # autres jobs tournent sur `stable`, qui satisfait n'importe quel plancher. Une dépendance
  # qui monte sa MSRV cassait donc les installations sans que la CI le voie.
  #
  # Un `check` et non la suite de tests : la question posée est « ce plancher compile-t-il ? »,
  # à quoi le typage répond en entier. `--locked` en plus, pour que le job échoue sur la
  # résolution versionnée plutôt que d'en inventer une autre.
  msrv:
    name: MSRV 1.94
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7

      - uses: dtolnay/rust-toolchain@1.94

      - uses: Swatinem/rust-cache@v2

      - name: cargo check
        run: cargo check --workspace --all-features --all-targets --locked
```

- [ ] **Étape 6 : vérifier que le YAML reste valide**

```bash
python3 -c "import yaml; d=yaml.safe_load(open('.github/workflows/ci.yml')); print(sorted(d['jobs']))"
```

Attendu : `['audit', 'linux', 'msrv', 'portabilite']`.

- [ ] **Étape 7 : rejouer localement les deux jobs**

```bash
cargo audit
cargo +1.94 check --workspace --all-features --all-targets --locked
```

Attendu : sortie 0 pour l'un et `Finished` pour l'autre. GitHub Actions n'est pas exécutable
ici : le rapport doit le dire, et ne rien affirmer sur la CI elle-même.

- [ ] **Étape 8 : commit**

```bash
git add .cargo/audit.toml .github/workflows/ci.yml
git commit -m "ci: ajoute un audit des advisories et un job épinglé sur la MSRV"
```

---

## Vérifications finales

- [ ] `cargo test --workspace` — vert
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` — vert
- [ ] `cargo fmt --all --check` — vert
- [ ] `cargo audit` — sortie 0
- [ ] `cargo test --workspace --no-fail-fast -- --ignored` — vert sous PostgreSQL 18
- [ ] `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))"` — sans erreur
