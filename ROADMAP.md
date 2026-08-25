# Roadmap — rbs (Rust Backend Starter)

> 🇬🇧 An English version of this roadmap lives at `docs/` once the documentation site is up.

`rbs` donne aux développeurs backend Rust un socle et un outillage pour démarrer une API
HTTP de production sans réécrire la même plomberie à chaque projet.

**Stack** : Rust · Axum · SeaORM · utoipa/Swagger · PostgreSQL

Le projet livre deux choses indissociables : un runtime (`rbs-core`) qui porte le
boilerplate invisible, et un CLI (`rbs`) qui génère le code que tu vas lire et modifier.

Le design complet est dans [`docs/superpowers/specs/2026-08-25-rbs-design.md`](docs/superpowers/specs/2026-08-25-rbs-design.md).
Les tâches en cours sont dans [`TODO.md`](TODO.md).

---

## Principes

Quatre partis pris qui expliquent la plupart des décisions du projet.

**Le code métier t'appartient.** `rbs-core` porte ce qui n'a aucune raison de varier
d'un projet à l'autre — erreurs, logs, configuration, middlewares. Tout ce que tu
voudras lire ou modifier est généré dans ton projet, en clair, sans macro à déplier.

**Pas de magie.** Le CLI branche les features par des ancres en commentaires que tu
peux voir et déplacer. Si une ancre manque, rbs ne devine pas : il te dit quoi coller
et où. Aucune réécriture d'AST, aucun reformatage de tes fichiers.

**Une architecture, une seule.** Chaque feature suit le même moule —
`model · dto · repository · service · controller` — avec une règle de dépendance
stricte : `controller → service → repository → model`. Le dixième développeur qui
arrive sur le projet lit le même code que le premier.

**Un starter qui génère du code sans tests enseigne à ne pas en écrire.** Chaque CRUD
généré arrive avec ses tests d'intégration.

---

## Jalons

### v0.1 — Socle

La chaîne complète, prouvée de bout en bout sur un seul cas avant d'être multipliée.

- `rbs new`, `rbs generate crud`, `rbs generate feature`
- `rbs add docker | ci`, `rbs migrate`, `rbs doctor`
- PostgreSQL, migrations SeaORM
- Erreurs typées (RFC 9457), logs colorés en dev / JSON en prod, configuration validée au boot
- OpenAPI et Swagger UI générés avec le code
- Documentation FR/EN, dépôt public, CI

**Critère de sortie** — un tiers clone, installe, génère une API CRUD qui tourne, *sans
poser de question*. Tant qu'une explication de vive voix est nécessaire pour démarrer,
la v0.1 n'est pas terminée.

### v0.2 — Auth

`rbs add auth` : JWT, Argon2, refresh tokens, middleware d'authentification, guards de
rôles, migration `users`.

**Critère de sortie** — une API protégée, générée de bout en bout.

### v0.3 — Intégrations

`rbs add redis`, `rbs add mail`, `rbs add storage`.

**Critère de sortie** — trois features suivant le même moule, ajoutées sans toucher au
noyau. Si l'une d'elles oblige à modifier `rbs-core`, c'est le moule qui est à revoir.

### v0.4 — Confort

Seeds, `rbs dev` (rechargement à chaud), jobs en arrière-plan, support MySQL et SQLite.

### v1.0 — Stabilité

Publication sur crates.io, semver, CHANGELOG, `rbs upgrade`.

**Critère de sortie** — l'API publique de `rbs-core` est figée. Jusque-là, aucune
promesse de compatibilité n'est faite.

---

## Hors périmètre

Explicitement, et non pas « plus tard » :

GraphQL · multi-tenancy · WebSockets · gRPC · interface d'administration générée ·
gestion des paiements

Un starter qui tente de tout couvrir ne couvre rien proprement. Ces sujets sont mieux
servis par des crates dédiées que par un générateur généraliste.

---

## État

| Jalon | Statut |
|---|---|
| v0.1 Socle | 🚧 en cours |
| v0.2 Auth | ⏳ planifié |
| v0.3 Intégrations | ⏳ planifié |
| v0.4 Confort | ⏳ planifié |
| v1.0 Stabilité | ⏳ planifié |
