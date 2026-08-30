---
sidebar_position: 8
title: rbs upgrade
---

# `rbs upgrade`

Aligne un projet engendré sur la version du CLI qui le lit : la dépendance `rbs-core` et la
version consignée dans `[package.metadata.rbs]`. Il affiche ensuite les notes de migration
que le saut traverse.

Il n'écrit que dans `Cargo.toml`, et dans les deux zones réservées d'
[`AGENTS.md`](../guides/agents.md) — nulle part ailleurs. Le reste du projet —
contrôleurs, configuration, migrations, et tout ce que vous écrivez hors de ces deux
zones — vous appartient dès l'instant où [`rbs new`](./new.md) l'a posé, et le re-rendre
sur une version plus récente effacerait votre travail sans que vous l'ayez demandé
nommément. Le guide, lui, est différent : c'est du texte que rbs produit et versionne, si
bien qu'un projet mis à niveau doit recevoir le mode d'emploi de la version qu'il fait
tourner désormais, plutôt que de continuer à lire celui de la version qu'il vient de
quitter.

:::note
Les blocs de terminal de cette page sont des sorties réelles, capturées en lançant la
commande. Elles sont identiques à celles de la page anglaise : le CLI parle français, une
sortie de terminal ne se traduit pas.
:::

## Synopsis

```text
$ rbs upgrade --help
Aligne le manifeste du projet sur la version du CLI : rbs-core et les métadonnées

Usage: rbs upgrade [OPTIONS]

Options:
      --force                  Met à niveau même si le working tree Git est sale
      --template-dir <CHEMIN>  Répertoire de templates remplaçant celles embarquées dans le binaire
  -y, --yes                    Prend les valeurs par défaut sans rien demander : le CLI reste scriptable
  -h, --help                   Print help
  -V, --version                Print version
```

`--force` est son seul flag propre. Les deux options globales sont acceptées parce que clap
les propage, et aucune n'a d'effet ici.

## Un saut qui porte une note

Un projet engendré par rbs 0.4.0, mis à niveau par un CLI en 1.0.0 :

```text
$ rbs upgrade
rbs 0.4.0 → 1.0.0

plan pour /private/tmp/rbs-demo/demo

  ~ Cargo.toml   modifié
  ~ AGENTS.md    modifié

  2 fichiers à écrire
✓ manifeste aligné sur rbs 1.0.0

# rbs 1.0.0 — la surface publique du noyau est gelée

22 types de `rbs-core` portent désormais `#[non_exhaustive]` : les 7 enums (`Error`,
`ConfigError`, `JwtError`, `LogError`, `Status`, `Check`, `LogFormat`) et 15 structs —
les 5 de la configuration, plus `ProblemDetails`, `Health`, `Checks`, `CoreState`,
`ConnectError`, `Identity`, `Pagination`, `Page<T>`, `JsonFormat` et `PrettyFormat`.

Deux choses cessent donc de compiler dans votre code :

- un `match` exhaustif sur un de ces enums réclame un bras `_ =>` ;
- une de ces structs construite par un littéral hors du noyau ne l'est plus : passez par
  son constructeur (`Page::new`) ou par la configuration désérialisée.

`Claims`, `ValidatedJson<T>` et `CommonResponses` en sont exclus, et c'est délibéré : le
code qu'`rbs new` et `rbs generate` écrivent les construit ou les déstructure. Un projet
engendré traverse la version sans une ligne à changer.

  cargo update -p rbs-core, puis cargo test
```

Code de sortie 0. Quatre choses s'y produisent, dans cet ordre : le saut est nommé, le plan
est affiché avant qu'un octet ne soit écrit, le manifeste est aligné, et la note de chaque
version que le saut traverse est affichée.

Une note appartient à la version qu'elle introduit, et non à un couple de versions. Une
mise à niveau qui enjambe plusieurs versions les affiche toutes — sinon la rupture apportée
par une version intermédiaire disparaîtrait au seul motif qu'on ne s'y est pas arrêté. Un
projet déjà passé par une note ne la revoit jamais.

La dernière ligne n'est pas un ornement. Le manifeste ne fait qu'énoncer la version voulue ;
tant que le fichier de verrouillage n'a pas suivi, le projet compile encore contre l'ancien
noyau.

## Le manifeste, et les zones d'AGENTS.md

Juste après la mise à niveau ci-dessus, dans le même projet :

```text
$ git diff --name-only
AGENTS.md
Cargo.toml
```

Deux fichiers, et c'est ce qui rend la promesse de [la page de
compatibilité](../compatibility.md) vérifiable à la main :

```text
$ git diff -U1 Cargo.toml
diff --git a/Cargo.toml b/Cargo.toml
index 5fd2f9c..7cf0dc8 100644
--- a/Cargo.toml
+++ b/Cargo.toml
@@ -12,3 +12,3 @@ default-run = "demo"
 [package.metadata.rbs]
-version = "0.4.0"
+version = "1.0.0"
 features = ["health"]
@@ -25,3 +25,3 @@ path = "src/seeds/main.rs"
 [dependencies]
-rbs-core = { version = "0.4.0", default-features = false, features = ["postgres"] }
+rbs-core = { version = "1.0.0", default-features = false, features = ["postgres"] }
 anyhow = "1.0"
```

Deux lignes : la métadonnée et la dépendance. Un noyau pris d'un chemin local, par
`rbs new --core-path` — le mode dans lequel rbs se développe — garde son chemin, une
dépendance par chemin n'ayant pas de version à monter ; seule la version consignée bouge.

```text
$ git diff -U2 AGENTS.md
diff --git a/AGENTS.md b/AGENTS.md
index 9064e97..77cfedc 100644
--- a/AGENTS.md
+++ b/AGENTS.md
@@ -1,6 +1,6 @@
 # demo — mode d'emploi pour agents

-<!-- rbs:guide 0.4.0 -->
+<!-- rbs:guide 1.0.0 -->
 ## Le CLI d'abord

 Ce projet est engendré par rbs. **Toute fonctionnalité que rbs couvre passe par le CLI**,
```

Seule la version du marqueur d'ouverture bouge ici — le texte du guide lui-même ne change
que le jour où une version le reformule vraiment. La zone d'inventaire, en dessous, est
elle aussi recalculée à chaque mise à niveau, mais ne produit son propre diff que si les
fragments, les entités ou le moteur de base du projet ont changé depuis la dernière
écriture : régénérer un contenu identique n'est pas un octet écrit.

Ce que vous avez ajouté hors des deux zones — le titre, `## Notes du projet`, tout ce qui
vous appartient — reste intact, comme le reste du projet. C'est le contrôle `agents` de
[`rbs doctor`](./doctor.md) qui aurait signalé un guide périmé avant que cette mise à
niveau ne tourne.

L'écriture passe par le même journal que toute commande touchant un projet existant : si
une écriture échoue en cours de route, ce qui avait déjà été écrit est défait.

## Un saut qui ne traverse aucune note

Toutes les versions ne rompent pas quelque chose. Un saut qui ne traverse aucune note le
dit plutôt que de se taire — un blanc là où une note était attendue se lit comme un échec :

```text
$ rbs upgrade
rbs 0.3.0 → 0.4.0

plan pour /private/tmp/rbs-demo/demo

  ~ Cargo.toml   modifié
  ~ AGENTS.md    modifié

  2 fichiers à écrire
✓ manifeste aligné sur rbs 0.4.0

  aucune note de migration pour rbs 0.3.0 → 0.4.0

  cargo update -p rbs-core, puis cargo test
```

Code de sortie 0.

## Rien à faire

Relancée une seconde fois sur le même projet, la commande s'arrête avant le plan :

```text
$ rbs upgrade
✓ le projet est déjà en rbs 0.4.0 — rien à faire
```

Code de sortie 0, et pas un octet écrit — pas même une réécriture de `Cargo.toml` à
l'identique. La commande se met sans risque dans un script qui la lance à chaque checkout.

C'est aussi pourquoi la garde Git vient après le plan et non avant : un projet qui n'a rien
à écrire n'a rien à protéger, et doit pouvoir répondre « rien à faire » depuis un working
tree plein de votre travail en cours.

## Un projet postérieur au CLI

```text
$ rbs upgrade
erreur : le projet est en rbs 1.0.0, le CLI en 0.4.0 : `rbs upgrade` ne redescend pas un projet — relancez-le avec un CLI en 1.0.0 ou plus récent
```

Code de sortie 1. `rbs upgrade` ne fait jamais redescendre un projet. Le cas courant est
celui de deux CLI installés côte à côte — un `cargo install` et un `cargo run` dans un
clone — et c'est pourquoi le message nomme les deux numéros : c'est la seule façon de
savoir lequel des deux vient d'être lancé.

Un numéro de version qui ne se réduit pas à trois nombres n'est pas tenu pour postérieur.
Mettre à niveau un manifeste qu'on ne sait pas lire est la moindre des deux erreurs.

## Un working tree sale

```text
$ rbs upgrade
erreur : le working tree n'est pas propre : src/main.rs, src/router.rs — commitez, ou relancez avec --force
```

Code de sortie 1. Une mise à niveau qui a quelque chose à écrire réclame un arbre propre,
pour que la ligne qu'elle change reste discernable des vôtres au prochain `git diff`.
Commitez, ou passez `--force` pour la forcer. Seuls les fichiers suivis comptent, et cinq
au plus sont nommés avant que le message ne dise combien il y en a d'autres — un arbre
comptant des centaines de modifications noierait le message dans ce qu'il est censé rendre
lisible.

Hors d'un dépôt Git, la garde ne trouve rien de modifié et laisse passer la mise à niveau :
il n'y a pas d'historique à protéger.

## Après la mise à niveau

`rbs upgrade` s'arrête au manifeste, et sa dernière ligne nomme les deux commandes qui
achèvent le travail : `cargo update -p rbs-core`, puis `cargo test`. La première fait
suivre le fichier de verrouillage à la version qui vient d'être demandée, la seconde dit si
les notes ci-dessus concernent votre code.

Puis [`rbs doctor`](./doctor.md), dont le contrôle `versions` compare la version consignée,
la dépendance `rbs-core` et le CLI qui diagnostique — les trois numéros que cette commande
vient d'aligner.
