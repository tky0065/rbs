# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Stack

Vue 3 + TypeScript, Vite, Pinia, Vue Router, Tailwind v4 (CSS-first), shadcn-vue
vendorisé (composants copiés dans les templates, jamais installés par un CLI tiers).
Décidé avec l'utilisateur : une SPA unique, `/` public et `/admin/*` en chunk paresseux
derrière une garde de route. Servie en production par Axum via `tower-http::ServeDir`
avec fallback SPA ; en développement par Vite avec proxy vers l'API Rust.

Cette arborescence est *déposée* par un binaire Rust (`rbs add frontend`) qui n'a ni
réseau ni Node : tout fichier livré est du texte UTF-8 rendu par minijinja, sans aucune
exécution de commande.

## Users

Un développeur backend Rust qui vient de générer un projet avec `rbs new` puis d'y
ajouter le fragment `frontend`. Terminal ouvert, il lance `cargo run` et ouvre
`localhost:3000`. Il se méfie des générateurs de frontend et jugera l'ensemble du projet
sur cette première page. Second public, le même développeur en tant qu'opérateur :
il ouvre `/admin` pour se connecter et regarder les données de son API.

## Product Purpose

`rbs` est un générateur de projets d'API web en Rust (Axum + SeaORM). Le fragment
`frontend` lui ajoute, en une commande hors-ligne, une application Vue complète et déjà
vraie : la page d'accueil décrit l'API générée à côté d'elle, et l'administration fournit
un shell authentifié prêt à être étendu.

Le succès : le développeur ouvre la page, ne la vide pas, et l'étend.

## Positioning

Le mécanisme qu'aucun voisin ne peut copier honnêtement : le frontend est déposé par un
binaire Rust sans réseau et sans Node, et son contenu par défaut est **vrai au premier
`cargo run`** — nom du projet, état réel de `/health`, routes réelles, lien vers la
documentation OpenAPI réellement servie. Aucun texte de remplissage mensonger.

## Operating Context

- Le projet engendré expose déjà : `/health` (sondes), `/docs` et `/api-docs/openapi.json`
  (documentation OpenAPI), et les routes des fragments installés.
- Le fragment `auth` (requis par `admin`) rend un `TokenPair` JSON — jeton d'accès JWT
  HS256 de 15 min, jeton de rafraîchissement opaque de 30 jours — transporté en
  `Authorization: Bearer`. Aucun cookie, aucune session serveur.
- Routes d'authentification réelles : `POST /auth/register`, `/auth/login`, `/auth/refresh`,
  `/auth/logout`, `GET /auth/me`, `/auth/change-password`, `/auth/forgot-password`,
  `/auth/reset-password`, `/auth/verify-email`, `/auth/resend-verification`,
  `GET|DELETE /auth/sessions`, `DELETE /auth/sessions/{id}`.
- Le fragment `cors` existe et n'autorise aucune origine par défaut ; le développement sur
  `localhost:5173` exige que l'utilisateur y déclare son origine.
- `rbs generate client --lang ts` produit déjà un client TypeScript typé depuis l'OpenAPI :
  le frontend n'a pas à réécrire sa couche HTTP.

## Capabilities and Constraints

Contraintes dures du mécanisme qui livre ces fichiers :

- **UTF-8 uniquement, aucun binaire.** Pas de favicon `.ico`, pas de police `.woff2`, pas
  d'image matricielle. SVG, CSS et code seulement.
- **Aucun hook.** Le CLI ne peut lancer ni `npm install` ni `npm run build`. Le `dist/`
  n'existe pas au premier `cargo run`, et la page ne s'affiche qu'après une installation
  manuelle des dépendances.
- **Rendu minijinja sur chaque fichier livré**, variables en `{@ … @}` ; `{{ }}` de Vue
  traverse intact, mais tout `{%` ou `{#` dans un `.vue` casse la génération.
- **Comparaison octet à octet** : l'exemple versionné qui portera ce fragment est comparé
  à ce que le CLI reproduit, donc chaque fichier livré est figé.
- Deux fragments distincts : `frontend` (socle + accueil, sans dépendance) et `admin`
  (`requires = ["frontend", "auth"]`).
- Jeton d'accès conservé en mémoire (Pinia), jeton de rafraîchissement en `localStorage` —
  compromis assumé et à documenter, faute de flux cookie `HttpOnly` côté serveur.

Hors périmètre, décidé : **aucune interface d'administration engendrée par entité**.
`rbs generate crud` n'émettra jamais de Vue (`ROADMAP.md:154`). L'administration est un
squelette que l'utilisateur étend en recopiant une page.

## Brand Commitments

Aucune. Le projet `rbs` n'a aujourd'hui aucune identité visuelle : son site de
documentation est un Docusaurus de série, palette verte par défaut jamais modifiée
(`docs/src/css/custom.css:9`), favicon de série. Rien à préserver.

Le nom se met en bas de casse : `rbs`. Documentation et interface bilingues français /
anglais (`--lang fr|en`).

**Aucun emoji dans le frontend engendré.** Contrainte posée par le mainteneur, sans
extension : l'iconographie passe exclusivement par de vraies icônes vectorielles — jeu
`@lucide/vue` dans l'application Vue, SVG inline partout où npm n'est pas disponible
(la page d'amorçage servie par Rust). Un emoji n'est jamais un substitut d'icône.

## Evidence on Hand

- Cinq projets d'exemple réels compilés en CI sous `examples/` ; un sixième portera le
  frontend.
- Captures et sorties réelles du CLI dans `docs/static/img/logs-pretty.png`.
- **À ne pas fabriquer** : aucun client, aucun témoignage, aucun chiffre d'adoption,
  aucune mesure de performance, aucun tarif. Le projet est une crate open source publiée
  sur crates.io (`rbs-core`, `rbs-cli`) — rien d'autre n'est établi.
