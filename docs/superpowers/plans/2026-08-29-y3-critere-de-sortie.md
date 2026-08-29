# Y3 — Critère de sortie du jalon

**Conception.** Trois vérifications, dont une seule demande un scénario complet.

**La première est l'acrobatie que la conception §2.6 assume**, et sa preuve ne pouvait venir
qu'après la publication : un projet engendré par la **0.4.0 installée depuis crates.io**,
amené à 1.0.0 par `rbs upgrade`, dont `cargo test` est vert après. Elle exige donc de
désinstaller la 1.0.0 pour remettre la 0.4.0, d'engendrer, puis de réinstaller la 1.0.0 —
c'est exactement le parcours d'un utilisateur qui monte de version, et rien de moins ne le
prouverait.

L'inversion est bornée parce qu'`upgrade` n'écrit que dans un manifeste : un manifeste faux
se corrigerait par une 1.0.1. Aucune des deux autres formes de la commande n'aurait eu cette
propriété.

**`cargo test` du projet engendré demande une base**, PostgreSQL 14 ou plus — le plancher
que `doctor` fait respecter, et non le 18 que la documentation annonçait à tort avant `Y1`.
Un conteneur suffit.

**La deuxième vérification mesure la cohérence du numéro publié.** `cargo semver-checks` de
1.0.0 contre 0.4.0 doit rendre un verdict qui s'accorde avec la montée majeure : la rupture
des `#[non_exhaustive]` est déclarée par le numéro, donc acceptée. Un verdict rouge ici
voudrait dire que le numéro ment.

**La troisième est une lecture**, et elle a déjà été faite en `W3` : le `README` annonce la
promesse. Elle se revérifie ici contre une 1.0.0 réellement parue, ce qui n'était pas le cas
quand `W3` l'a écrite.

## Étapes

1. `cargo install rbs-cli --version 0.4.0 --force`, puis `rbs new` un projet : il porte
   `rbs-core = "0.4.0"` et `[package.metadata.rbs].version = "0.4.0"`, résolus depuis le
   registre.
2. Réinstaller la 1.0.0, lancer `rbs upgrade`, lire la note de migration affichée.
3. Démarrer PostgreSQL en conteneur, `rbs migrate up`, puis `cargo test` du projet — vert.
4. `cargo semver-checks --package rbs-core --all-features` sur le dépôt, verdict lu.
5. Relire la phrase du `README` dans les deux langues.
6. Preuves : chaque sortie lue, non supposée. Le projet doit avoir traversé la version
   **sans une ligne de source modifiée** — c'est ce que la note promet.
