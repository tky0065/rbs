# Roadmap — rbs (Rust Backend Starter)

> 🇬🇧 An English version of this roadmap lives at `docs/` once the documentation site is up.

`rbs` donne aux développeurs backend Rust un socle et un outillage pour démarrer une API
HTTP de production sans réécrire la même plomberie à chaque projet.

**Stack** : Rust · Axum · SeaORM · utoipa/Swagger · PostgreSQL 14+ (MySQL 8.0+, SQLite 3.35+)

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
- PostgreSQL 14+, MySQL 8.0+ ou SQLite 3.35+ — les modèles posent eux-mêmes
  l'identifiant v7, aucun moteur n'a à connaître `uuidv7()` ; migrations SeaORM
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

Conception :
[`docs/superpowers/specs/2026-08-27-v0.2-auth-design.md`](docs/superpowers/specs/2026-08-27-v0.2-auth-design.md).
Le jalon tient deux chantiers et non un : les primitives d'auth, et le moule qui permet à
une feature d'apporter du code Rust — `rbs add` ne sait aujourd'hui installer que des
fragments qui n'en apportent pas. C'est ce moule que le critère de sortie de la v0.3
jugera.

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

### v1.1 — Agents

Un `AGENTS.md` posé dans chaque projet engendré : le mode d'emploi de rbs écrit pour un
agent. Deux zones délimitées appartiennent au CLI — le guide, versionné, et l'inventaire du
projet ; tout le reste du fichier appartient au développeur et n'est jamais réécrit.
`add`, `generate` et `upgrade` le tiennent à jour, `doctor` le contrôle. Guide bilingue,
choisi par `rbs new --lang` ou déduit de la locale.

La règle « le CLI d'abord » cesse d'être déclarative : `doctor` nomme le code qui n'est pas
passé par le CLI — en avertissement, jamais en échec, parce qu'écrire à la main ce que rbs
ne couvre pas reste légitime.

**Critère de sortie** — un agent partant d'un projet fraîchement engendré produit une
feature complète en passant par le CLI, `rbs doctor` restant vert.

Les versions qui suivent n'ont pas été planifiées comme des jalons à critère de sortie :
elles viennent des backlogs ouverts après la v1.1, et le détail de chacune est dans
`CHANGELOG.md`.

### v1.2 — Intégrations sortantes

Le client TypeScript typé (`rbs generate client --lang ts`) et `rbs new --preset
api|worker|full`. Six fragments de plus : `webhooks` (abonnements signés, livraison par
les jobs), `scheduler` (déclenchement calendaire), `audit` (journal des écritures),
`observability` (traces OTLP et `/metrics`), `cors`, `rate-limit`. Côté CRUD :
`--with-upload`, `--soft-delete` et la pagination par curseur (`Cursor`, `CursorPage` dans
`rbs-core`). Une ancre `<rbs:layers>` pour empiler les middlewares, et un
`config/production.toml` qui ferme Swagger UI.

### v1.3 — Routes fermées

Sur un projet qui porte `auth`, `rbs generate crud` écrit des routes fermées ;
`require_role` compare un seuil au lieu d'une égalité ; `rbs add` range ses modules sous
`src/modules/`. La 1.3.1 documente chaque colonne d'un filtre engendré par son type.

### v1.4 — Parcours de compte

Le fragment `auth` passe de cinq routes à treize : changement et réinitialisation du mot
de passe, vérification de l'adresse, sessions listées et fermées. Une table
`one_time_tokens` partagée, un garde `VerifiedIdentity`, et `auth` exige désormais `mail`.

### v1.5 — Robustesse

L'arrêt gracieux sur Ctrl-C ou SIGTERM, un worker de jobs concurrent dont le délai de
reprise croît, `--singular` pour les pluriels irréguliers, la fermeture du SSRF des
webhooks, les correctifs de sécurité d'`auth` (rejeu des jetons, révocation effective,
adresses normalisées, parcours transactionnels), le dépôt atomique de `storage`, et
`rbs new --lang` qui fixe désormais la langue des réponses HTTP (`[server] lang`).

### v1.6 — Retrait et clés d'API

`rbs remove <feature>` défait ce qu'`add` a posé — fichiers, lignes d'ancre, migration et
dépendances devenues orphelines — dans l'ordre inverse de l'installation, et refuse avant
d'écrire sur quatre motifs. `rbs add api-keys` authentifie les machines plutôt que les
personnes : une clé présentée dans `X-Api-Key` satisfait `Identity` partout où un jeton de
session le fait, sans qu'une ligne d'un CRUD engendré plus tôt change.

### v1.7 — Frontend

Deux fragments posent une application Vue 3 dans le projet : `frontend` livre le socle —
routeur, deux stores Pinia, thème Tailwind v4, quatorze composants shadcn-vue figés dans
les templates — et une page d'accueil dont le contenu par défaut est vrai au premier
démarrage : elle interroge `/health`, nomme le projet et renvoie à sa propre documentation
OpenAPI. `frontend-admin` y ajoute un shell authentifié de cinq écrans, chacun adossé à
une route que le fragment `auth` expose réellement. Un module `src/modules/frontend/` sert
le build en production et, tant qu'il n'existe pas, une page d'amorçage qui nomme les
commandes manquantes — le CLI n'ayant aucun moyen de lancer `npm` lui-même.

Ce jalon **modifie le hors-périmètre ci-dessous** : `rbs generate crud` émet désormais,
en plus de l'entité et de sa migration, les écrans d'administration de la table, dès lors
que `frontend-admin` est posé — sur le même principe qu'il écrit des routes fermées dès que
`auth` l'est, acquis en v1.3. Deux ancres côté TypeScript portent l'insertion, la table de
routage et le rail, ce qui monte le registre à vingt. Un `--no-admin` reste la sortie de
secours. Motif et renversement en `docs/adr/0003`.

### v1.8 — Le projet engendré s'ouvre et se lance

Un projet créé avec les seize fragments s'ouvrait sur une impasse : aucun chemin engendré ne
produisait un compte capable d'entrer dans l'espace d'administration, quand l'écran de
connexion affirmait que les identifiants étaient ceux de la table des comptes — une table
vide. Le fragment `auth` dépose désormais un seed qui crée le premier compte à partir de
deux variables tirées au hasard à l'installation et rendues par ses `next_steps` :
administrateur, adresse déjà vérifiée, sans effet si le compte existe ou si les variables
sont vides, et refusé sous `RBS_ENV=production`. Le shell d'administration gagne les trois
pages publiques qui lui manquaient — inscription, réinitialisation, vérification d'adresse
—, et le compte devient modifiable.

Le squelette pose un `Makefile` dont les raccourcis n'appellent que `cargo`, `npm` et
`docker compose`, jamais le générateur : un collègue qui clone le dépôt lance `make dev`
sans installer le CLI. Deux ancres nouvelles portent ces insertions — `# <rbs:make>`, et
`// <rbs:vite_proxy>` où `generate crud` inscrit le préfixe de route de la table, faute de
quoi l'écran qu'il vient d'écrire ne chargerait rien en développement —, ce qui monte le
registre à vingt-deux. Lire le contrat devient instantané : il est mémorisé sous la cible
de build, invalidé par un condensat des sources, et `rbs routes` comme `rbs generate
client` prennent un `--from <FICHIER>` pour une CI sans toolchain Rust. Enfin la couche de
transport du frontend descend du shell vers le socle, dont elle est le prérequis et non le
complément — `docs/adr/0004`.

---

## Hors périmètre

Explicitement, et non pas « plus tard » :

GraphQL · multi-tenancy · WebSockets · gRPC · gestion des paiements

Un starter qui tente de tout couvrir ne couvre rien proprement. Ces sujets sont mieux
servis par des crates dédiées que par un générateur généraliste.

**L'interface d'administration générée a quitté cette liste le 2026-09-19.** Elle y
figurait depuis l'origine ; le jalon v1.7 la fait entrer dans le périmètre, `rbs generate
crud` émettant désormais ses écrans d'administration quand le fragment `frontend-admin`
est posé. Le renversement et son motif sont consignés en `docs/adr/0003`, qui abroge
`docs/adr/0001`. Une liste de hors-périmètre qui perd une entrée sans le dire est une
liste à laquelle on ne peut plus se fier.

---

## État

| Jalon | Statut |
|---|---|
| v0.1 Socle | ✅ livré |
| v0.2 Auth | ✅ livré |
| v0.3 Intégrations | ✅ livré |
| v0.4 Confort | ✅ livré — publiée sur crates.io le 2026-08-28 |
| v1.0 Stabilité | ✅ livré — publiée le 2026-08-29, API de `rbs-core` figée |
| v1.1 Agents | ✅ livré — publiée le 2026-08-30 ; critère de sortie non encore éprouvé sur un agent réel |
| v1.2 Intégrations sortantes | ✅ livré — publiée le 2026-09-04 |
| v1.3 Routes fermées | ✅ livré — publiée le 2026-09-08, corrigée par la 1.3.1 le 2026-09-09 |
| v1.4 Parcours de compte | ✅ livré — publiée le 2026-09-11 |
| v1.5 Robustesse | ✅ livré — publiée dans la 1.6.0, le numéro 1.5.0 n'ayant jamais été tagué |
| v1.6 Retrait et clés d'API | ✅ livré — publiée le 2026-09-19 |
| v1.7 Frontend | ✅ livré — publiée le 2026-09-20 |
| v1.8 Le projet engendré s'ouvre | ✅ livré le 2026-09-20 — pas encore publié |
