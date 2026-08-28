# `rbs dev` — orchestration du démarrage

**But :** une seule commande remonte ce qu'il faut, applique les migrations et lance le
serveur, sans que le développeur ait à se rappeler l'ordre.

**Approche :** la séquence est d'abord **planifiée**, puis exécutée — le motif que `add`
et `generate` suivent déjà. `plan()` lit le manifeste et le `.env`, et rend une
`Vec<Step>` ; rien n'est lancé pendant la lecture. C'est ce qui rend la première preuve
possible sans Docker : *aucun compose cherché* se constate sur un plan, pas sur une trace.

Le compose n'est cherché que si `docker` figure dans `[package.metadata.rbs].features`.
L'ordre des deux conditions compte et se teste : la sonde d'existence de fichier est
injectée, et le test vérifie qu'elle n'est **jamais appelée** sur un projet sans la
feature. Un `exists()` d'abord, un `features.contains()` ensuite passerait un test qui
ne regarderait que le plan produit.

L'attente de la base réutilise la sonde TCP de `doctor::base` : le CLI ne parle pas plus à
la base ici qu'ailleurs. La sonde est elle aussi injectée, pour que l'échec se prouve en
quelques millisecondes plutôt qu'en un délai réel.

**Écarté :** appeler `docker compose up` sans condition et laisser Docker échouer — le
message serait celui de Docker, sur un projet qui n'a jamais demandé de conteneur.

**Hors périmètre :** le watch, qui est `Q2`. `dev` lance ici le serveur une fois.

## Fichiers

| Chemin | Rôle |
|---|---|
| `crates/rbs-cli/src/dev/mod.rs` | `Step`, `plan()`, `run()`, `Error` |
| `crates/rbs-cli/src/cli.rs` | variante `Dev` |
| `crates/rbs-cli/src/lib.rs` | bras `Commands::Dev` |

## Étapes

- [ ] `Step` et `plan()` : compose conditionné à la feature, attente, migrations, serveur.
- [ ] `wait_for()` : sonde répétée jusqu'à échéance, erreur nommant hôte et port.
- [ ] `run()` : exécution des étapes, `docker compose up -d` puis `migrate::launch(up)`.
- [ ] Câblage `cli.rs` / `lib.rs`, code de sortie 1 sur échec via `ui::error`.

## Preuves attendues

- ✓ *Projet sans la feature `docker` → aucun compose cherché, démarrage quand même* —
  test unitaire sur `plan()` avec sonde d'existence instrumentée.
- ✓ *Base injoignable → message nommant ce qui manque* — `wait_for()` avec sonde
  toujours fausse rend un `Err` dont le `Display` porte l'hôte, le port et la variable.
