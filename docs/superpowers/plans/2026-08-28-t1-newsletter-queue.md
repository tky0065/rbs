# T1 — L'exemple `newsletter-queue`

**Conception.** Un quatrième exemple, sur le motif des trois en place : ce que les commandes
produisent, plus les éditions à la main qui sont le point de l'exemple. Il alimente les
pages que `T2` écrira, et rien d'autre ne le justifie.

Le fragment `mail` en fait partie, contre la parenthèse de la tâche qui le laissait ouvert
« au prix d'un conteneur de plus en CI ». Deux raisons, dans cet ordre :

- Le prix n'existe pas. Le step `examples/` de la CI boucle sur `examples/*/` et n'y lance
  que `cargo clippy` — aucun test, aucun service. `file-drop` porte déjà `mail` sans qu'un
  conteneur ait été ajouté pour lui.
- `T2` l'exige. Sa page de `mail` doit « montrer le passage à un job », et aucun de ses
  extraits ne peut être écrit à la main. Le passage `send_detached` → job doit donc être
  lisible **dans l'exemple**.

L'ordre d'installation n'est pas libre : l'ancre `<rbs:features>` empile les `mod` dans
l'ordre où les features arrivent et doit rester triée, faute de quoi `cargo fmt` bronche
dans le projet engendré. `jobs`, puis `mail`, puis `subscribers` — ce qui écarte
`newsletter` comme nom de ressource. `email` seul suffit à la contrainte de validation du
DTO (`generate/fields.rs:109`) ; le nommer `subscriber_email` n'ajouterait rien.

Ce que l'exemple montre et que les trois autres ne montrent pas : un job enfilé **dans la
transaction** qui l'a motivé. C'est l'argument qui a fait choisir une table contre Redis en
`R1`, resté jusqu'ici dans un test d'intégration.

## Étapes

1. Générer le projet par les trois commandes, en commitant entre chaque `add`.
2. Les éditions du README qu'aucune commande ne produit : `.git` supprimé, `rbs-core` en
   chemin relatif, marqueurs `// region:`, `.env` forcé au suivi.
3. Éditions à la main, cinq fichiers : `src/jobs/newsletter.rs` (le job), `src/jobs/mod.rs`
   (le registre, dont `demo::Log` sort), `src/subscribers/{service,controller,mod}.rs`
   (la route `broadcast` qui enfile dans la transaction), `templates/mail/newsletter.html`,
   et le seed qui peuple les abonnés.
4. `integration_examples.rs` : une quatrième entrée dans `EXEMPLES` avec sa liste
   `edite_a_la_main`, le test de dérive, et `the_hand_edits_of_newsletter_queue_are_in_place`
   — sans lequel la liste d'exclusion serait une porte ouverte à la dérive qu'elle déclare.
5. `examples/README.md` : la ligne du tableau et la section de régénération.
6. Preuves : `cargo clippy --workspace --all-targets -- -D warnings` dans l'exemple, comme
   la CI ; `cargo test -p rbs-cli --test integration_examples` ; morsure d'une édition à la
   main retirée, qui doit faire tomber le test des éditions et lui seul.

La CI ne bouge pas : sa boucle prend le nouvel exemple d'elle-même — c'est explicitement
pourquoi elle est écrite en boucle plutôt qu'en un step par exemple.
