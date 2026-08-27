# `doctor` — signaler un noyau déclaré depuis crates.io alors que rbs n'y est pas publié

> **For agentic workers:** REQUIRED SUB-SKILL: `superpowers:executing-plans`.

**Goal :** sur un projet qui déclare `rbs-core = "<version>"` comme dépendance de
registre, `rbs doctor` rend `✗ versions` et donne le geste qui débloque, au lieu du `✓`
d'alignement des numéros qu'il rend aujourd'hui.

**Architecture :** le contrôle `versions` ne peut pas savoir sans réseau si une version de
crates.io est résoluble ; il sait en revanche, à la compilation du CLI, si son noyau est
publié. Une constante porte ce fait, et la décision est extraite dans une fonction qui la
reçoit en paramètre — sans quoi le chemin « publié » resterait à jamais inatteignable donc
non testé, et basculerait la constante sur quatre tests cassés d'un coup.

**Spec :** `docs/superpowers/plans/2026-08-27-v1-frictions.md`, friction **D4**.

## Contraintes globales

- Verdict retenu : `Etat::Echec`. Aucun troisième état n'est ajouté au modèle.
- Le constat de non-publication **prime** sur l'écart de numéros : sans résolution
  possible, comparer deux versions n'apprend rien.
- `Noyau::Local` reste `✓` — c'est le parcours du guide de démarrage, qui aboutit.
- Le remède est non destructif : le lecteur a déjà un projet, « regénérez » lui ferait
  perdre son travail.
- Ne coche pas `V1`. Lève sa dernière friction connue.

## Fichiers

- Modifier : `crates/rbs-cli/src/doctor/versions.rs` — seul fichier touché.
- Tests : le module `tests` du même fichier.

Vérifié avant d'écrire ce plan : les quatre tests d'intégration génèrent avec
`--core-path` (`tests/common/mod.rs:45`, `integration_crud.rs:58`, `integration_new.rs:23`,
`integration_examples.rs:54`) donc restent au vert, et `docs/docs/cli/doctor.md:53` montre
déjà `rbs-core pris d'un chemin local` : la doc reste exacte, aucune page à reprendre.

---

### Tâche unique : le contrôle `versions` distingue la source du noyau

**Interfaces :**
- Produit : `controler(&Path) -> Controle` (signature publique inchangée) et, en interne,
  `controler_avec(racine: &Path, noyau_publie: bool) -> Controle`.

- [ ] **Étape 1 — écrire les tests qui échouent**

Les quatre premiers portent le nouveau comportement, le cinquième est le garde-fou du
parcours qui marche.

```rust
#[test]
fn un_noyau_de_registre_est_signale_tant_que_rbs_n_est_pas_publie() {
    let (_parent, racine) = projet();

    let controle = controler_avec(&racine, false);

    assert_eq!(controle.etat, Etat::Echec, "{}", controle.detail);
    assert!(controle.detail.contains("crates.io"), "{}", controle.detail);
    assert!(controle.detail.contains(CLI));
}

#[test]
fn le_remede_donne_le_chemin_local_a_declarer() {
    let (_parent, racine) = projet();

    let controle = controler_avec(&racine, false);
    let remede = controle.remede.expect("un échec porte son remède");

    assert!(remede.contains("path"), "{remede}");
    assert!(remede.contains("crates/rbs-core"), "{remede}");
}

#[test]
fn la_non_publication_prime_sur_l_ecart_de_numeros() {
    let (_parent, racine) = projet();
    reecrire(&racine, &format!("rbs-core = \"{CLI}\""), "rbs-core = \"0.0.1\"");

    let controle = controler_avec(&racine, false);

    assert_eq!(controle.etat, Etat::Echec);
    assert!(controle.detail.contains("crates.io"), "{}", controle.detail);
    assert!(controle.detail.contains("0.0.1"), "{}", controle.detail);
}

#[test]
fn une_fois_le_noyau_publie_un_projet_neuf_est_coherent() {
    let (_parent, racine) = projet();

    let controle = controler_avec(&racine, true);

    assert_eq!(controle.etat, Etat::Bon, "{}", controle.detail);
    assert!(controle.detail.contains(CLI));
}

#[test]
fn une_fois_le_noyau_publie_un_ecart_de_numeros_reste_signale() {
    let (_parent, racine) = projet();
    reecrire(&racine, &format!("rbs-core = \"{CLI}\""), "rbs-core = \"0.0.1\"");

    let controle = controler_avec(&racine, true);

    assert_eq!(controle.etat, Etat::Echec);
    assert!(controle.detail.contains("rbs-core"));
    assert!(controle.detail.contains("0.0.1"));
}
```

Deux tests existants changent de prémisse et sont réécrits, pas supprimés :
`un_projet_neuf_est_coherent_avec_le_cli_qui_l_a_genere` devient
`une_fois_le_noyau_publie_un_projet_neuf_est_coherent` (sa prémisse est fausse
aujourd'hui) ; `un_noyau_d_une_autre_version_que_le_cli_est_signale` se scinde en ses deux
cas de publication. `un_projet_genere_par_une_autre_version_est_signale_avec_les_deux_numeros`
passe par un noyau local, pour isoler l'écart de numéros du blocage de résolution.
`un_noyau_pris_d_un_chemin_local_est_dit_sans_etre_tenu_pour_fautif` et
`un_manifeste_sans_dependance_au_noyau_est_signale` restent inchangés.

- [ ] **Étape 2 — les voir échouer**

`cargo test -p rbs-cli doctor::versions` → échec de compilation, `controler_avec` n'existe pas.

- [ ] **Étape 3 — implémenter**

```rust
/// Faux tant que `rbs-core` n'est pas sur crates.io : un projet qui l'y déclare ne résout
/// pas, et `doctor` est le seul endroit où le lecteur bloqué peut l'apprendre.
const NOYAU_PUBLIE: bool = false;

pub(crate) fn controler(racine: &Path) -> Controle {
    controler_avec(racine, NOYAU_PUBLIE)
}
```

Dans `controler_avec`, après résolution du `Noyau` et avant la comparaison des numéros :

```rust
Noyau::Version(version) if !noyau_publie => {
    return Controle::echec(
        TITRE,
        format!("rbs-core {version} déclaré depuis crates.io, où rbs n'est pas encore publié"),
        "clonez https://github.com/tky0065/rbs, puis dans Cargo.toml :\n\
         rbs-core = { path = \"<clone>/crates/rbs-core\" }",
    );
}
```

- [ ] **Étape 4 — les voir passer**

`cargo test -p rbs-cli doctor::versions`, puis `cargo test --workspace`,
`cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check`.

- [ ] **Étape 5 — preuve manuelle du rendu**

Générer un projet sans `--core-path` dans un répertoire temporaire, y lancer `rbs doctor`,
et lire la sortie réelle : c'est elle que le lecteur bloqué verra.

- [ ] **Étape 6 — commit, puis annotation `PARTIEL` de `V1` mise à jour**
