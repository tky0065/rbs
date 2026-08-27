# Répétition à blanc du critère de sortie

**But** — trouver les frictions mécaniques du parcours « clone → installe → API CRUD qui
tourne » avant de mobiliser une personne extérieure, dont le regard neuf est une ressource
qui ne se dépense qu'une fois.

## Protocole

1. **Parcours A — le lecteur du README.** Suivre `README.md` et lui seul, littéralement.
   C'est le chemin par défaut depuis GitHub.
2. **Parcours B — le lecteur du site.** Suivre `docs/docs/getting-started.md` mot à mot.

Règle : n'exécuter que ce que la page prescrit. Une commande qui échoue est une friction
**même si le correctif est connu** — le corriger pour avancer, mais la compter.

## Isolation

Répertoires sous le scratchpad ; `cargo install --root` dédié et `PATH` préfixé, pour ne
toucher ni à `~/.cargo/bin` ni au `rbs` de Ruby présent sur la machine ; PostgreSQL en
conteneur sur un port non standard.

## Critère d'arrêt

« Une API qui tourne » se prouve par le réseau : serveur lancé, `POST /articles` → 201,
`GET /articles` → 200 renvoyant l'article créé. `cargo build` ne suffit pas.

## Sortie

Journal de frictions, chacune rédigée en tâche de documentation (fichier, ligne,
correctif). `V1` reste `- [ ]` avec annotation `PARTIEL` : le critère nomme une personne
extérieure au projet, et cette répétition ne trouve que les frictions mécaniques — pas les
frictions cognitives, hors de portée de qui connaît déjà la réponse.
