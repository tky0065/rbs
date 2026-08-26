# Test d'intégration CRUD — plan

**But :** prouver, par un test, que la chaîne complète tient : un projet créé par le
binaire livré, une feature CRUD générée, sa migration appliquée, et les tests générés qui
passent contre un vrai PostgreSQL 18.

**Approche :** le conteneur démarre en premier. Son port n'est connu qu'une fois lancé, et
c'est lui qui détermine l'URL passée à `rbs new` — l'ordre inverse obligerait à réécrire
le `.env` après coup, ce qu'un utilisateur ne fait pas.

L'attente porte sur le message de PostgreSQL compté **deux fois** : le serveur l'émet une
première fois pendant son initialisation, alors qu'il n'accepte pas encore de connexion
extérieure. Attendre la première occurrence produit un test qui échoue une fois sur trois.
Les deux flux sont suivis ensemble : sur `stderr` seul, le compte à deux n'aboutit pas —
Docker ne répartit pas les deux annonces sur le même flux.

Rien n'est simulé : le binaire `rbs` est invoqué comme un utilisateur l'invoquerait, et le
`cargo test` final est celui du projet généré.

**Hors périmètre :** les cas d'erreur de chaque commande, déjà couverts en tests unitaires.
Ce test prouve le chemin nominal de bout en bout, et rien d'autre.

## Fichiers

| Chemin | Rôle |
|---|---|
| `crates/rbs-cli/tests/common/mod.rs` | `depot()`, `noyau()`, `cible()`, partagés |
| `crates/rbs-cli/tests/integration_crud.rs` | le test |
| `crates/rbs-cli/tests/integration_new.rs` | bascule sur le module commun |

## Étapes

- [x] `tests/common/mod.rs`, et `integration_new.rs` qui s'y branche sans changer de
      comportement.
- [x] Le conteneur PostgreSQL 18 et l'URL qu'il impose.
- [x] Les quatre étapes enchaînées, chacune vérifiée.
- [x] Preuve du rouge : une étape cassée volontairement doit faire échouer le test.

## Preuves attendues

- ✓ *Rouge si l'une des trois étapes échoue* — échec provoqué sur la génération, puis sur
  la migration, avec la sortie du test dans les deux cas.
- Vert dans le cas nominal.
- Non régression : `cargo test --workspace -- --include-ignored`, `clippy -D warnings`,
  `fmt --check`.
