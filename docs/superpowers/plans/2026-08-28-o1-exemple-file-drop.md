# `examples/file-drop` — plan d'implémentation

> **Pour un exécutant :** dérouler tâche par tâche, chaque étape dans l'ordre.
> Les cases suivent l'avancement.

**But :** un exemple portant les trois features du jalon v0.3 — `redis`, `mail`,
`storage` — câblées dans un CRUD `uploads`, compilé par la CI et comparé à une
génération fraîche par `integration_examples`.

**Approche :** l'infrastructure existe et reste intacte. La boucle
`for exemple in examples/*/` de `ci.yml` prend le nouvel exemple sans modification du
workflow ; `integration_examples` gagne une entrée dans `EXEMPLES` et deux `#[test]`.
Le travail neuf est l'exemple lui-même et son câblage.

**Pile :** Axum, SeaORM, `deadpool-redis`, `lettre`, `aws-sdk-s3`, `assert_cmd`.

**Design :** approuvé en chat le 2026-08-28 (tâche `O1` de `TODO.md`).

**Note du 2026-08-28, après la tâche 1 :** les identifiants du dépôt sont passés à
l'anglais entre la tâche 1 et la tâche 2. Les signatures citées plus bas sont celles
d'après la migration — voir
`docs/superpowers/plans/2026-08-28-glossaire-migration-anglais.md`.

## Contraintes globales

- L'ancre `features` empile les `mod` dans l'ordre d'installation et doit rester
  triée : `cache` < `mail` < `storage` < `uploads`. `rbs add redis` insère
  `mod cache;`, non `mod redis;`.
- Les commandes s'exécutent depuis la racine du dépôt, le CLI par
  `cargo run -p rbs-cli --bin rbs --`.
- `rbs add` refuse un working tree sale : commit dans le projet généré avant la
  première feature.
- Trois éditions communes à tout exemple : supprimer le `.git` que `rbs new`
  initialise, réécrire `rbs-core` en `{ path = "../../crates/rbs-core" }`, forcer
  `git add -f .env` que le `.gitignore` du projet ignore.
- Les marqueurs `// region:` sont hors comparaison (`est_marqueur`) : les poser ne
  coûte aucune exclusion.
- `#![warn(missing_docs)]` ne concerne que `rbs-core` — l'exemple est un projet
  utilisateur, ses commentaires n'expliquent que le *pourquoi*.

---

### Tâche 1 : l'exemple brut ne dérive pas

Le socle : l'exemple existe, il est exactement ce que le CLI produit, et le test de
dérive le constate. Aucun câblage à ce stade — une régression ici serait invisible
sous les éditions à la main.

**Fichiers :**
- Créer : `examples/file-drop/` (généré)
- Modifier : `crates/rbs-cli/tests/integration_examples.rs` (`EXEMPLES` + un `#[test]`)

**Interfaces :**
- Consomme : `Exemple { nom, database_url, features, crud, champs, edite_a_la_main }`,
  `verifier_non_derive`, `engendrer` — tous déjà génériques, aucun à modifier.
- Produit : l'entrée `file-drop` dans `EXEMPLES`, lue par la tâche 2.

- [ ] **Étape 1 : écrire le test qui échoue**

Dans `EXEMPLES`, après `blog-auth` :

```rust
    Exemple {
        nom: "file-drop",
        database_url: "postgres://rbs:rbs@localhost:5432/file_drop",
        // `mod cache;` — non `mod redis;` — puis `mail`, `storage` : l'ancre empile
        // dans l'ordre d'installation, et `uploads` est le seul nom de ressource qui
        // la laisse triée derrière `storage`.
        features: &["redis", "mail", "storage"],
        crud: "uploads",
        // `owner_email` finit par `_email` : le DTO généré gagne sa contrainte
        // d'email sans qu'on l'écrive, et le courriel a un destinataire qui vient
        // du modèle.
        champs: "title:string,owner_email:string,content_type:string,size:int",
        edite_a_la_main: &[],
    },
```

et le test :

```rust
#[test]
fn file_drop_est_celui_que_le_cli_produit_aujourd_hui() {
    verifier_non_derive(exemple("file-drop"));
}
```

- [ ] **Étape 2 : le voir échouer**

`cargo test -p rbs-cli --test integration_examples file_drop`
Attendu : FAILED — `examples/file-drop` n'existe pas.

- [ ] **Étape 3 : engendrer l'exemple**

```bash
cargo run -p rbs-cli --bin rbs -- new file-drop --yes \
  --core-path ./crates/rbs-core \
  --database-url 'postgres://rbs:rbs@localhost:5432/file_drop'
cd file-drop && git add -A && git commit -q -m 'projet neuf'
for f in redis mail storage; do
  cargo run --manifest-path ../Cargo.toml -p rbs-cli --bin rbs -- add "$f" --yes
done
cargo run --manifest-path ../Cargo.toml -p rbs-cli --bin rbs -- \
  generate crud uploads --fields 'title:string,owner_email:string,content_type:string,size:int' \
  --yes --force
cd .. && mv file-drop examples/file-drop
```

Puis les trois éditions communes : `rm -rf examples/file-drop/.git`, `rbs-core` ramené
à `{ path = "../../crates/rbs-core" }` dans `examples/file-drop/Cargo.toml`, et
`git add -f examples/file-drop/.env`.

- [ ] **Étape 4 : le voir passer**

`cargo test -p rbs-cli --test integration_examples file_drop` → PASS.

- [ ] **Étape 5 : compiler, et figer les dépendances**

`cd examples/file-drop && cargo clippy --workspace --all-targets -- -D warnings`,
puis `cargo fmt --all --check`. Le `Cargo.lock` écrit par cette compilation est
versionné — la CI compile l'exemple à dépendances figées — et reste hors comparaison.

- [ ] **Étape 6 : commit**

```bash
git add examples/file-drop crates/rbs-cli/tests/integration_examples.rs
git commit -m "test(examples): un exemple portant cache, courriel et stockage"
```

---

### Tâche 2 : le câblage, et le test qui le tient

Ce qu'aucune commande ne produit. Les trois fragments livrent une brique sans route et
le disent : « la ligne se retire au premier appel », « leurs appelants sont les
handlers à écrire », « la permission tombe avec la première route qui dépose ou sert
un fichier ». C'est ici que ces phrases deviennent vraies.

**Fichiers :**
- Modifier : `examples/file-drop/src/uploads/service.rs`,
  `examples/file-drop/src/uploads/controller.rs`,
  `examples/file-drop/src/uploads/tests.rs`,
  `examples/file-drop/src/{cache,mail,storage}/mod.rs`
- Créer : `examples/file-drop/templates/mail/depot.html`
- Modifier : `crates/rbs-cli/tests/integration_examples.rs`
  (`edite_a_la_main` + `les_editions_a_la_main_de_file_drop_sont_en_place`)

**Interfaces consommées** (relevées dans les fragments, à ne pas deviner) :
- `Cache::get<T: DeserializeOwned>(&self, key) -> Result<Option<T>>`,
  `Cache::set<T: Serialize + ?Sized>(&self, key, &T) -> Result<()>`,
  `Cache::invalidate_prefix(&self, prefix) -> Result<usize>`
- `Mailer::message(&self, recipient, subject, body) -> Result<Message>`,
  `Mailer::send_template<S: Serialize>(&self, recipient, subject, template, context) -> Result<()>`,
  `Mailer::send_detached(&self, message)`
- `Storage::put(&self, key, Vec<u8>)`, `Storage::get(&self, key) -> Vec<u8>`,
  `Storage::delete(&self, key)`, `Storage::exists(&self, key) -> bool`
- L'état porte les champs publics `state.cache`, `state.mail`, `state.storage` ;
  `state.rs` reste intact, c'est là que les ancres se lisent.

- [ ] **Étape 1 : le gabarit du dépôt**

`examples/file-drop/templates/mail/depot.html`, sur la forme de `bienvenue.html` que
le fragment livre (contexte `{ title, link }`).

- [ ] **Étape 2 : écrire les tests qui échouent**

Dans `examples/file-drop/src/uploads/tests.rs`, le cycle : un dépôt relu à
l'identique, une suppression qui retire l'objet du stockage, et la liste servie par le
cache après un premier appel. Dans `integration_examples.rs`,
`les_editions_a_la_main_de_file_drop_sont_en_place` — sur la forme de
`les_editions_a_la_main_de_blog_auth_sont_en_place` — assertant :

```rust
for module in ["cache", "mail", "storage"] {
    let source = lire(&format!("src/{module}/mod.rs"));
    assert!(
        !source.contains("allow(dead_code)"),
        "src/{module}/mod.rs : la permission tombe avec le premier appel, \
         et c'est ce que cet exemple montre"
    );
}
```

plus la présence de `invalidate_prefix`, `send_detached` et `put` dans
`src/uploads/service.rs`.

- [ ] **Étape 3 : les voir échouer**

`cd examples/file-drop && cargo test` puis
`cargo test -p rbs-cli --test integration_examples editions_a_la_main_de_file_drop`.
Attendu : FAILED des deux côtés.

- [ ] **Étape 4 : câbler**

`service.rs` : `list` lit `uploads:page:{n}` avant la base et l'y écrit après ;
`create` dépose le contenu, invalide `uploads:`, envoie le gabarit en détaché ;
`update` et `delete` invalident ; `delete` retire aussi l'objet du stockage.
`controller.rs` : `PUT /uploads/{id}/content` et `GET /uploads/{id}/content`, corps
en `text/plain` — pas de DTO à changer, pas de base64, aucune dépendance neuve.
Retirer les trois `allow(dead_code)` des `mod.rs` de features.

- [ ] **Étape 5 : les voir passer**

`cd examples/file-drop && cargo test` → PASS, puis clippy et fmt propres.

- [ ] **Étape 6 : déclarer les exclusions**

`edite_a_la_main` de l'entrée `file-drop` reçoit les sept chemins. Relancer
`cargo test -p rbs-cli --test integration_examples file_drop` → PASS : la comparaison
ignore exactement ce que le test d'étape 2 asserte, et rien d'autre.

- [ ] **Étape 7 : commit**

```bash
git add examples/file-drop crates/rbs-cli/tests/integration_examples.rs
git commit -m "feat(examples): câble le cache, le courriel et le stockage sur uploads"
```

---

### Tâche 3 : la régénération se documente

Un exemple dont la commande de régénération n'est pas écrite dérive au premier
changement de template : `REGENERER` renvoie à `examples/README.md`, qui doit tenir
la promesse.

**Fichiers :**
- Modifier : `examples/README.md`

- [ ] **Étape 1 : la table et la section de régénération**

Ligne `file-drop` dans la table des projets, section « Regenerating » reprenant les
commandes de la tâche 1, et la liste des sept éditions à la main sous « Edits the CLI
does not produce » — une phrase par édition, disant *pourquoi* elle existe.

- [ ] **Étape 2 : rejouer la régénération depuis le README seul**

Suivre les commandes écrites, à la lettre, dans un répertoire temporaire, puis
comparer à `examples/file-drop`. Ce que le README omet se voit ici, et nulle part
ailleurs.

- [ ] **Étape 3 : commit**

```bash
git add examples/README.md
git commit -m "docs(examples): consigne la régénération de file-drop"
```

---

### Preuves à produire pour cocher `O1`

| Critère `✓` de `TODO.md` | Commande |
|---|---|
| Compilé par le step `examples/` de la CI, sur les trois plateformes | La boucle `for exemple in examples/*/` de `ci.yml` — voir la réserve ci-dessous |
| `integration_examples` le compare à une génération fraîche | `cargo test -p rbs-cli --test integration_examples` |

**Réserve sur le premier critère, à arbitrer avant de cocher.** Le step
`cargo clippy (examples)` n'existe que dans le job `linux`. Le job `portabilite`
(macOS, Windows) lance `cargo clippy --workspace` et `cargo test --workspace`, et le
manifeste racine **exclut** les exemples. Aucun exemple n'est donc compilé ailleurs
que sous Linux — `hello-crud` et `blog-auth` compris. `ci.yml` le dit et l'argumente :
« les contrôles de dépôt — format, exemples, dry-run de publication — ne dépendent pas
de la plateforme ; les tripler coûterait des minutes de runner sans rien démontrer de
plus ».

Le critère et le workflow se contredisent. Deux issues, et c'est au propriétaire du
backlog de trancher :

1. **Étendre** : ajouter la boucle au job `portabilite`, `shell: bash` pour que Windows
   l'exécute. Le critère est alors tenu au sens littéral, contre la décision inscrite
   dans `ci.yml`.
2. **Corriger le critère** : les exemples restent sur Linux, et `O1` se coche sur un
   critère réécrit. C'est un changement de critère, pas une preuve — il se demande.

Tant qu'aucune n'est retenue, `O1` reste `- [ ]` avec une annotation `PARTIEL` : le
second critère est prouvable dès aujourd'hui, le premier ne l'est pas.
