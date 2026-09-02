# La prose de la documentation garantie par un test — design

**Date** : 2026-09-02
**Tâches couvertes** : `IMPROVE.md` 123, 109, 99.

## Le problème

Quatre blocs de `docs/docs/getting-started.md` et `docs/docs/cli/new.md` ont vécu périmés
sur trois axes à la fois — compte de fichiers du squelette, nombre de features connues,
fichiers déposés par `auth` et par `generate crud` — et la prose raisonnait sur leurs
chiffres. Quatre autres pages citent des transcripts `rbs doctor` annonçant onze ancres
quand `anchors.rs` en porte douze. Les deux guides d'observabilité citent des blocs écrits
à la main, quand `CLAUDE.md` pose que la documentation ne cite aucune ligne écrite à la
main et tire ses extraits d'`examples/`.

Rien ne surveille cela : `docs/scripts/parite.mjs` ne voit que la structure et les liens,
jamais les nombres ; `integration_examples` ne couvre que le code d'`examples/`, pas les
sorties citées en prose.

## Le principe

Un bloc de transcript est un **oracle** : il dit ce qu'une commande rend. Un oracle qui
n'est jamais rejoué se périme sans bruit. On lui donne donc la même garantie qu'au code
d'`examples/` — un test qui rejoue la commande et compare octet à octet, à la
normalisation près.

## Architecture

### Le marqueur

Chaque bloc gardé est précédé d'un commentaire MDX, invisible au rendu Docusaurus.
Un commentaire HTML ne convient pas : Docusaurus compile ces pages en MDX, qui refuse
`<!--` — le site ne se construit plus.

```markdown
{/* rbs:transcript cmd="rbs new demo-api --database-url postgres://rbs:rbs@localhost:5432/demo" */}
```text
✓ demo-api créé — 18 fichiers
```
```

Le marqueur porte la commande, et elle seule. Le bloc qui suit immédiatement est la
sortie attendue. Un bloc sans marqueur n'est pas gardé : l'adoption est progressive, et
un bloc illustratif (une sortie tronquée, un exemple de configuration) doit pouvoir
rester tel quel.

Attributs reconnus :

| Attribut | Rôle | Défaut |
|---|---|---|
| `cmd` | la commande à rejouer, telle qu'un utilisateur la tape | obligatoire |
| `setup` | une commande à jouer avant, pour poser le décor (séparées par ` && `) | aucune |
| `dans` | le sous-répertoire du tmpdir où lancer `cmd` | la racine du tmpdir |
| `base` | `oui` si la commande exige un PostgreSQL joignable | `non` |
| `extrait` | `oui` si le bloc est une portion de la sortie et non son intégralité | `non` |

### La normalisation

Ce qui varie d'une exécution à l'autre ne peut pas être comparé. Le test remplace, dans
la sortie comme dans le bloc attendu :

- le chemin du répertoire temporaire, par `<tmp>` ;
- toute durée (`in 0.11s`, `en 1.2 s`), par `<durée>` ;
- la version du CLI, par `<version>` ;
- la version du moteur de base (`postgres 18.6`), par `<moteur>` ;
- les séquences ANSI, effacées — le binaire est lancé sans TTY, il n'en écrit pas, mais
  la garantie ne coûte rien.

La comparaison porte sur les lignes non vides, espaces de fin retirés.

### Où vit le test

`crates/rbs-cli/tests/integration_docs.rs`, à côté d'`integration_examples.rs`, dont il
reprend le rituel : il extrait, rejoue, compare, et nomme la page et la ligne du bloc qui
diverge. Les blocs `base="oui"` sont rassemblés dans un test `#[ignore]` distinct, comme
tout ce qui exige Docker dans ce dépôt.

Les pages parcourues sont `docs/docs/**/*.md` **et** `docs/i18n/fr/docusaurus-plugin-content-docs/current/**/*.md` :
une jumelle française qui dérive est une jumelle qui ment.

### Ce qui n'entre pas

Le test ne rejoue pas les commandes des guides qui exigent un service tiers (un collecteur
OTLP, un SMTP) : ces blocs restent hors garde, et la tâche 99 les traite autrement — en
tirant leurs extraits d'un exemple compilé en CI plutôt qu'en les rejouant.

## Les trois tâches

**123** pose le marqueur, l'extracteur, la normalisation, le test, et marque les quatre
blocs de `getting-started.md` et `cli/new.md` plus leurs jumelles.

**109** rejoue `rbs doctor` sur un projet neuf avec PostgreSQL, reprend transcript **et**
prose ensemble sur `docs/docs/getting-started.md`, `docs/docs/cli/doctor.md`, leurs
jumelles françaises et le JSON `docs/i18n/fr/…/cli/doctor.md`, puis marque ces blocs
`base="oui"`.

**99** installe `observability` sur `examples/newsletter-queue` — un projet déjà compilé
en CI et déjà couvert par le test de non-dérive — met à jour ses deux `README` et fait
tirer aux deux guides leurs extraits de ce code, comme les autres pages le font déjà.

## Ce qui prouve que c'est fait

- `cargo test -p rbs-cli --test integration_docs` : vert, et rouge si l'on change un
  chiffre dans un bloc marqué.
- `cargo test -p rbs-cli --test integration_docs -- --ignored` : vert, Docker monté.
- `cargo test -p rbs-cli --test integration_examples` : vert après régénération de
  `newsletter-queue`.
- `node docs/scripts/parite.mjs` : aucun écart.
