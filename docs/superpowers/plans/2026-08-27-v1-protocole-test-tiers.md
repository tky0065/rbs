# Protocole du test du critère de sortie

Le critère de `V1` :

> Une personne extérieure au projet clone, installe, génère une API CRUD qui tourne,
> **sans poser de question**. Chaque question posée devient une tâche de documentation
> avant que la v0.1 ne soit déclarée close.

Ce fichier tient en trois parties. La première ne se transmet pas au testeur : elle cite
les frictions déjà connues et le tracé attendu du parcours, et la lui montrer reviendrait
à lui donner les réponses. Les deux autres se copient telles quelles, au choix de sa
langue.

---

## Partie 1 — Pour l'observateur

**Ne pas transmettre.**

### Ce que le test mesure

Une seule chose : ce qu'un tiers ne comprend pas. Les frictions mécaniques — commande qui
échoue, prérequis absent, contradiction entre deux pages — ont déjà été trouvées par la
répétition à blanc et corrigées. Ce qui reste hors de portée du projet, c'est le moment où
quelqu'un lit une phrase exacte et en tire la mauvaise conclusion.

D'où la règle qui gouverne tout le protocole : **le testeur ne pose aucune question**. Non
pour le laisser en peine, mais parce qu'une question posée puis répondue disparaît sans
laisser de trace. Consignée, elle devient une tâche de documentation.

### Qui peut jouer ce test

- N'a jamais lu la documentation de rbs, ni participé à sa conception.
- Sait faire compiler un projet Rust. Le test porte sur rbs, pas sur l'installation de
  `rustup` — quelqu'un qui n'a jamais vu Rust produirait un journal illisible où les deux
  se mélangent.
- Peut faire tourner Docker, ou dispose déjà d'un PostgreSQL 18.
- Accepte de tenir un journal pendant l'essai, ce qui rallonge la session.

Ne convient pas : quelqu'un à qui le projet a déjà été raconté de vive voix. La
présentation orale répare en amont les frictions que le test cherche.

### Ce que le testeur reçoit

L'adresse du dépôt, la consigne ci-dessous, rien d'autre. Pas de lien direct vers le guide
de démarrage : depuis la correction de D1 et D2, le `README` ne porte plus le parcours mais
y renvoie, et **savoir si ce renvoi se suit fait partie de ce qui se mesure**. Lui donner
l'URL du guide, c'est répondre d'avance à la première question du test.

La consigne lui interdit `docs/superpowers/`. Le dossier contient
`2026-08-27-v1-frictions.md`, qui décrit les murs un par un.

### Frictions déjà closes

Elles ne comptent pas comme découvertes. Si l'une ressort, c'est une **régression**, à
traiter comme telle et non comme un enseignement du test :

| | Friction | Corrigée le |
|---|---|---|
| D1 | Le « Quick look » du `README` générait un projet qui ne compile pas | 2026-08-27 |
| D2 | Le `README` ne faisait jamais démarrer de base de données | 2026-08-27 |
| D3 | Le conflit de nom avec le `rbs` de Ruby n'était signalé que sur le site | 2026-08-27 |
| D4 | `rbs doctor` déclarait saine une dépendance introuvable | 2026-08-27 |
| D5 | Le `rbs` de Ruby capturait la commande, sans qu'aucun code puisse le signaler | 2026-08-27 |

D5 est la seule des cinq qui n'ait pas été trouvée par la répétition à blanc : elle
n'apparaît que sur une machine où Homebrew Ruby précède `~/.cargo/bin`. Le binaire est
désormais installé sous les deux noms, et un testeur bloqué là n'a plus qu'à taper
`rbs-cli`.

### Ce qui invalide une session

- Une question a été posée **et** répondue pendant l'essai. La réponse a modifié le
  parcours ; ce qui suit ne mesure plus rien.
- Le testeur a ouvert `docs/superpowers/`, `TODO.md` ou l'historique Git.
- Le parcours a été joué en présence de quelqu'un du projet, qui a pu guider sans le
  vouloir — un regard par-dessus l'épaule au bon moment vaut une réponse.

Une session interrompue par un blocage, en revanche, reste valide et instructive : c'est
un résultat, pas un échec du protocole.

### Dépouillement

Chaque ligne de la colonne « ce que je ne comprends pas » devient une tâche de
documentation. Sans exception et sans arbitrage sur sa légitimité : le testeur ne
comprenait pas, c'est le fait mesuré. Une question qui semble tenir à sa méconnaissance de
Rust plutôt qu'à rbs se note quand même, avec ce constat — la fréquence dira si elle mérite
une phrase dans le guide.

### Condition de cochage de V1

La case se coche si, **et seulement si** :

1. Une session valide a été jouée par une personne répondant au profil.
2. Le parcours a abouti — la collection répond avec l'article créé.
3. Toutes les tâches de documentation issues du dépouillement sont faites.

Le point 3 est celui qu'on oublie : le critère dit « avant que la v0.1 ne soit déclarée
close ». Un parcours réussi avec quatre questions en suspens ne coche pas `V1`, il produit
quatre tâches.

---

## Partie 2 — Consigne au testeur (français)

*Copier à partir d'ici.*

### Ce qu'on te demande

Partir de `https://github.com/tky0065/rbs` et arriver à une API CRUD qui répond sur ta
machine. Tu y arrives quand une requête `curl` sur la collection que tu auras créée te
renvoie l'enregistrement que tu viens d'y écrire.

Tu es libre du chemin. Il n'y a pas d'étapes imposées ici : trouver le chemin **est**
l'exercice.

### La règle qui compte

**Ne pose aucune question à personne** — ni à l'auteur du projet, ni à un collègue, ni à un
assistant IA. Quand une question te vient, écris-la dans ton journal et débrouille-toi
avec ce que le projet te donne.

Ce n'est pas une épreuve de solitude. Une question à laquelle on te répond s'évapore ; une
question écrite devient une correction de la documentation. C'est tout l'objet de la
séance : ce sur quoi tu butes est le résultat, pas ta capacité à t'en sortir.

Chercher sur le web est autorisé — un vrai utilisateur le fait. Note simplement ce que tu
es allé chercher, et pourquoi.

### Deux endroits à ne pas ouvrir

Le dossier `docs/superpowers/` du dépôt et le fichier `TODO.md` : ils contiennent les
notes de conception, et notamment la liste de ce qu'on s'attend à te voir rencontrer. Les
lire viderait la séance de son sens. L'historique Git non plus, pour la même raison : les
messages de commit racontent les corrections récentes. Tout le reste du dépôt et du site
est à toi.

### Si tu bloques

**Quinze minutes sur une même étape, puis tu notes et tu avances.** Si la suite est
possible sans avoir résolu le blocage, continue ; sinon, arrête-toi là et dis-le. Une
séance qui s'arrête à la troisième commande est un résultat utile — souvent plus utile
qu'une séance qui aboutit.

### Ce dont tu as besoin

- Rust stable installé, et de quoi compiler.
- Docker, ou un PostgreSQL 18 déjà en route.
- `curl`, ou n'importe quel client HTTP.

### Ton journal

Un fichier Markdown, une ligne par moment notable. Ne le rédige pas après coup : au fil de
l'eau, même en style télégraphique.

```markdown
| Heure | Ce que je fais | Ce que je ne comprends pas |
|-------|----------------|----------------------------|
| 14:02 | j'ouvre le README | c'est quoi la différence entre rbs-core et rbs-cli ? |
| 14:07 | je lance cargo install | il compile depuis 4 min, c'est normal ? |
```

La troisième colonne est celle qui sert. Remplis-la même quand tu as trouvé la réponse
deux minutes plus tard : ces deux minutes sont exactement ce qu'on cherche à supprimer.

À la fin, ajoute trois lignes : **es-tu arrivé au bout ?**, **combien de temps en tout ?**,
et **ce que tu n'as toujours pas compris en refermant**.

---

## Partie 3 — Instructions for the tester (English)

*Copy from here.*

### What we are asking

Start from `https://github.com/tky0065/rbs` and get to a CRUD API answering on your
machine. You are there when a `curl` request on the collection you created returns the
record you just wrote into it.

The route is yours to find. No steps are prescribed here: finding the route **is** the
exercise.

### The rule that matters

**Ask nobody anything** — not the project's author, not a colleague, not an AI assistant.
When a question comes to you, write it in your log and make do with what the project
gives you.

This is not an endurance test. A question someone answers evaporates; a question written
down becomes a documentation fix. That is the whole point of the session: what you get
stuck on is the result, not your ability to get unstuck.

Searching the web is allowed — a real user does. Just note what you went looking for, and
why.

### Two places not to open

The repository's `docs/superpowers/` directory and its `TODO.md`: they hold the design
notes, including the list of what we expect you to run into. Reading them would empty the
session of its purpose. Nor the Git history, for the same reason: the commit messages
narrate the recent fixes. Everything else in the repository and on the site is yours.

### If you get stuck

**Fifteen minutes on one step, then note it and move on.** If the rest is possible without
resolving the blocker, carry on; otherwise stop there and say so. A session that stops at
the third command is a useful result — often more useful than one that succeeds.

### What you need

- Rust stable installed, and a working compiler.
- Docker, or a PostgreSQL 18 already running.
- `curl`, or any HTTP client.

### Your log

A Markdown file, one line per notable moment. Do not write it up afterwards: as you go,
telegraphic style is fine.

```markdown
| Time  | What I am doing | What I do not understand |
|-------|-----------------|--------------------------|
| 14:02 | opening the README | what is the difference between rbs-core and rbs-cli? |
| 14:07 | running cargo install | it has been compiling for 4 min, is that normal? |
```

The third column is the one that serves. Fill it in even when you found the answer two
minutes later: those two minutes are exactly what we are trying to remove.

At the end, add three lines: **did you get there?**, **how long in total?**, and **what
you still did not understand as you closed it**.
