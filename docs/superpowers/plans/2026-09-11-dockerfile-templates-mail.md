# L'image Docker engendrée embarque `templates/` — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Sous `docker compose --profile app up`, un projet portant `mail` envoie ses courriels : l'étage runtime du `Dockerfile` engendré copie `templates/` à côté de `config/`.

**Architecture:** `mail/config.rs.jinja:33` lit `templates/mail` à l'exécution par un chemin relatif au répertoire de travail (`/app`). Le `Dockerfile.jinja:23-27` ne copie que les deux binaires et `config/`. On ajoute une ligne **inconditionnelle** `COPY --from=builder /build/templates* ./templates/` : BuildKit (le constructeur par défaut depuis Docker 23) tolère un motif sans correspondance — vérifié sur Docker 29.1.3 : sans `templates/`, l'étape passe sans rien créer ; avec, `templates/mail/…` arrive entier. L'inconditionnel tient quel que soit l'ordre dans lequel `mail` et `docker` ont été posés (`rbs new --with docker,mail` installe `docker` avant `mail`, et `rbs add mail` peut venir après), là où un `{% if "mail" in features %}` raterait ces deux cas.

**Tech Stack:** minijinja (`{@ @}` pour les valeurs), Dockerfile multi-étapes, tests de rendu dans `crates/rbs-cli/src/add/mod.rs`.

**Spec:** design validé en chat (tâche 2 d'`IMPROVE.md`) — tâche *bounded*, sans document de spec.

## Global Constraints

- Conventional Commits en français, sans identifiant de tâche, sans renvoi à `IMPROVE.md`, sans `Co-Authored-By` ni `Claude-Session`.
- Un commentaire dit le *pourquoi* ; celui du Dockerfile doit dire pourquoi le glob et pourquoi inconditionnel.
- Documentation bilingue : toute page EN modifiée l'est aussi en FR dans le même commit.
- Aucun exemple d'`examples/` ne porte `docker` : rien à régénérer.
- Le `.dockerignore.jinja` n'exclut pas `templates/` : ne pas y toucher.

---

### Task 1: La ligne `COPY`, son test de rendu, et une preuve BuildKit

**Files:**
- Modify: `crates/rbs-cli/templates/features/docker/Dockerfile.jinja:26-27` (après `COPY config ./config`)
- Modify: `crates/rbs-cli/src/add/mod.rs` (module `tests`, à côté du test existant qui rend `docker` — `grep -n '"docker"' crates/rbs-cli/src/add/mod.rs`)

**Interfaces:**
- Consumes: le helper `projected(&planned, "<chemin>")` des tests d'`add/mod.rs` (lit un fichier tel que le plan l'écrira) — voir son usage ligne ~1181.
- Produces: le `Dockerfile` rendu contient `COPY --from=builder /build/templates* ./templates/`.

- [x] **Step 1: Écrire le test de rendu qui échoue**

Dans `crates/rbs-cli/src/add/mod.rs`, module `tests`, à côté du test existant sur `docker` (chercher `features = ["health", "docker"]`, ligne ~812, et reprendre sa façon de construire `planned`) :

```rust
    /// Le fragment `mail` lit `templates/mail` à l'exécution : une image qui ne l'embarque
    /// pas n'envoie aucun courriel, et rien ne le dit avant le premier `register`.
    #[test]
    fn the_dockerfile_ships_the_templates_directory() {
        let (_parent, root) = project_with(&["health"]);  // adapter au helper réel du module
        let planned = plan(&root, "docker", None).expect("plan constructible"); // idem

        let dockerfile = projected(&planned, "Dockerfile");
        assert!(
            dockerfile.contains("COPY --from=builder /build/templates* ./templates/"),
            "l'étage runtime n'embarque pas templates/ :\n{dockerfile}"
        );
    }
```

Remplacer les deux premières lignes par la construction exacte qu'emploie le test voisin (lire ce test avant d'écrire).

- [x] **Step 2: Le voir échouer**

```bash
cargo test -p rbs-cli --lib -- the_dockerfile_ships_the_templates_directory 2>&1 | tail -8
```

Attendu : `FAILED`, message « l'étage runtime n'embarque pas templates/ ».

- [x] **Step 3: Écrire la ligne dans la template**

Dans `Dockerfile.jinja`, après `COPY config ./config` :

```dockerfile
# `mail` lit `templates/mail` à l'exécution, par un chemin relatif à /app : sans cette
# couche, l'API journalise « envoi échoué » à chaque courriel et n'envoie rien. Le motif
# tient lieu de condition — BuildKit copie ce qu'il trouve et ne se plaint pas du reste —
# et la ligne vaut quel que soit l'ordre dans lequel `mail` et `docker` ont été posés,
# `rbs add mail` après coup compris.
COPY --from=builder /build/templates* ./templates/
```

- [x] **Step 4: Le voir passer, avec les voisins**

```bash
cargo test -p rbs-cli --lib -- add:: 2>&1 | tail -5
```

Attendu : tous verts, dont `the_dockerfile_ships_the_templates_directory`.

- [x] **Step 5: Preuve BuildKit sur le Dockerfile réellement rendu**

Rendre deux projets (avec et sans `mail`) et construire l'étage runtime seul, en substituant à l'étage `builder` une copie du contexte (compiler le projet dans l'image prendrait dix minutes et ne prouverait rien de plus sur le `COPY`) :

```bash
S=/private/tmp/claude-501/-Users-yacoubakone-dev-rs/20433d69-a7c1-492d-8d62-250f705907e5/scratchpad/dockerfile-mail
rm -rf $S && mkdir -p $S && cd $S
RBS="cargo run -q --manifest-path /Users/yacoubakone/dev/rs/Cargo.toml -p rbs-cli --bin rbs --"
$RBS new avec --yes --with docker,mail --database-url 'postgres://rbs:rbs@localhost:5432/avec' >/dev/null
$RBS new sans --yes --with docker --database-url 'postgres://rbs:rbs@localhost:5432/sans' >/dev/null
for p in avec sans; do
  # L'étage builder devient une copie brute du contexte, l'étage runtime est celui du rendu.
  awk 'BEGIN{b=1} /^FROM debian/{b=0} b&&/^FROM rust/{print "FROM alpine:3.20 AS builder\nWORKDIR /build\nCOPY . .\nRUN mkdir -p target/release && touch target/release/'$p' target/release/migration"; next} b{next} {print}' $p/Dockerfile > $p/Dockerfile.runtime
  (cd $p && docker build -q -f Dockerfile.runtime -t rbs-runtime-$p . >/dev/null && docker run --rm rbs-runtime-$p sh -c 'ls -R /app/templates')
done
```

Attendu : pour `avec`, la liste montre `/app/templates/mail:` puis `bienvenue.html` (et les autres gabarits du fragment) ; pour `sans`, un build qui réussit et aucun `/app/templates`. Consigner les deux sorties dans le message de commit. Nettoyer : `docker rmi rbs-runtime-avec rbs-runtime-sans`.

Note : le `Dockerfile.runtime` retire aussi le `USER api` ? Non — le laisser, `ls` fonctionne sous `api`. Si `debian:trixie-slim` n'est pas en cache local, le premier build le tire (quelques secondes).

- [x] **Step 6: Commit**

```bash
git add crates/rbs-cli/templates/features/docker/Dockerfile.jinja crates/rbs-cli/src/add/mod.rs
git commit -m "fix(docker): embarque templates/ dans l'image pour que mail envoie en conteneur" -m "<pourquoi : mail lit templates/mail par chemin relatif ; l'étage runtime ne copiait que config/ ; le preset api (auth + docker) n'envoyait aucun courriel sous compose. Glob inconditionnel plutôt qu'une condition sur la feature, qui raterait rbs new --with docker,mail et tout add mail postérieur.>" -m "Vérifications :
- cargo test -p rbs-cli --lib -- add:: : <N> passés
- docker build de l'étage runtime rendu, avec mail : /app/templates/mail/bienvenue.html présent ; sans mail : build réussi, /app/templates vide"
```

---

### Task 2: La documentation, si elle décrit l'image

**Files:**
- Vérifier : `docs/docs/cli/add.md:46,68-90` et FR (`docs/i18n/fr/docusaurus-plugin-content-docs/current/cli/add.md`), `docs/docs/guides/mail.md` et FR (`grep -n 'templates/mail\|conteneur\|container\|compose' …`).

- [x] **Step 1: Chercher ce que la doc dit du contenu de l'image et du chemin des gabarits**

```bash
grep -n 'templates/mail\|container\|conteneur\|compose' docs/docs/guides/mail.md docs/i18n/fr/docusaurus-plugin-content-docs/current/guides/mail.md
grep -n 'config/\|image' docs/docs/cli/add.md | sed -n 1,10p
```

- [x] **Step 2: Si `mail.md` parle du chemin relatif `templates/mail` sans dire ce qu'il implique en conteneur, ajouter une phrase EN + FR**

Exemple EN, à la suite du paragraphe qui nomme `templates/mail` :

```markdown
The path is relative to the working directory: the `Dockerfile` written by `rbs add docker`
copies `templates/` next to `config/` into the image, so the same configuration works
under `docker compose --profile app up`.
```

Et l'équivalent FR. Si aucune page ne décrit ni le chemin ni l'image, ne rien écrire — le dire dans le rapport.

- [x] **Step 3: Vérifier**

```bash
cargo test -p rbs-cli --test integration_docs 2>&1 | tail -5
cd docs && node scripts/parite.mjs 2>&1 | tail -5
```

Attendu : vert ; parité sans écart nouveau (l'écart `IMPROVE_OLD.md` préexiste, tâche 28 du backlog).

- [x] **Step 4: Commit (seulement si une page a changé)**

```bash
git commit -am "docs(mail): dit que l'image Docker embarque les gabarits de courriel" -m "Vérifications :
- cargo test -p rbs-cli --test integration_docs : vert
- node docs/scripts/parite.mjs : aucun écart nouveau"
```

---

### Task 3: Passe finale

- [x] **Step 1:**

```bash
cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -3 && cargo test -p rbs-cli --lib 2>&1 | tail -3
```

Attendu : fmt muet, clippy sans warning, tests lib verts.

- [x] **Step 2: Rapport** — branche, `git log --oneline main..HEAD`, et pour chaque preuve la ligne exacte lue.
