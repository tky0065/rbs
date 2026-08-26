# F3 — Démarrage rapide (FR + EN)

## Décision

La page est complète et exacte sur l'essentiel : elle traite déjà le `--core-path` rendu
nécessaire par la non-publication de `rbs-core`, et la collision de nom avec le `rbs` de
Ruby. Ce qui la bloquait tenait au dépôt, pas au texte — et le dépôt a changé.

Un seul passage est devenu faux. Un encart `:::caution` avertit que `main` « ne porte pas
toujours toutes les commandes » et renvoie le lecteur vers un « arbre de développement de
la 0.1 ». La vérification de F13 établit le contraire : le binaire installé depuis le
dépôt public expose `new`, `add`, `generate`, `migrate` et `doctor`. Un encart qui envoie
chercher un dépôt inexistant coûte plus qu'il ne protège — et c'est exactement la question
que le critère de sortie V1 interdit de faire naître.

## Étapes

1. Supprimer l'encart dans `docs/docs/getting-started.md` et dans son miroir français, le
   même commit portant les deux langues.
2. Rejouer la page à la lettre depuis un clone public, dans un répertoire vierge :
   `git clone`, `cargo install --path crates/rbs-cli`, PostgreSQL en conteneur,
   `rbs new --core-path`, `generate crud`, `migrate up`, `cargo run`, puis les appels
   HTTP sur `/health`, `POST /articles`, `GET /articles`, et `doctor`.

## Preuve

Le critère est « suivi à la lettre sur une machine vierge, sans intervention extérieure ».
Ce qui l'empêchait — un `origin/main` périmé — ne l'empêche plus.

Réserve à consigner : le clone porte `origin/main`, qui n'a pas les commits de CI et de
publication du site faits aujourd'hui. Ils ne touchent ni le CLI ni les templates, donc le
parcours reste valide ; l'écart est nommé dans la preuve plutôt que passé sous silence.
