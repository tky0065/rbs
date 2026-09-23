# Les références dans les écrans d'administration — conception

**Date** : 2026-09-23
**Portée** : dans les écrans que `rbs generate crud` émet sur un projet portant
`frontend-admin`, un champ `references` se **choisit** dans un sélecteur à recherche et se
**lit** par un libellé, au lieu d'un UUID saisi et affiché brut. Le fragment `auth` gagne la
route qui rend les comptes sélectionnables. Hors périmètre, explicitement : un tri par
libellé, une résolution côté serveur (jointure, champ `*_libelle` dans les réponses), un
sélecteur multiple, un contrôle de propriétaire sur les écritures.

---

## Pourquoi

`crates/rbs-cli/src/ecran.rs:675` range toute référence dans `Controle::Texte`. Tout projet
engendré en hérite : le formulaire demande de coller un UUID, la table et le détail en
affichent un. Constaté sur `examples/help-desk` : pour commenter un ticket, il faut copier
son identifiant depuis l'écran des tickets ; la colonne « Auteur id » ne dit pas qui a écrit.
Retoucher l'exemple ne corrigerait qu'un exemple, et chaque utilisateur referait la retouche :
le défaut est celui du générateur.

## Décisions

| Question | Décision |
|---|---|
| Cibles couvertes | Les tables engendrées **et** `users`. |
| Où se résout le libellé | Dans l'écran (approche A) : le contrat des API engendrées ne change pas. |
| Colonne libellé | Devinée — la première colonne textuelle de la cible — et surchargeable par `label=<colonne>`. |
| Droits sur les comptes | `POST /users/filter` réservé au rôle `admin`. |

## 1. Ce que le CLI décide, au plan

Pour chaque champ `references` d'un `generate crud` sur un projet portant `frontend-admin`,
le CLI établit une **fiche de référence** qu'il passe à l'écran :

| Élément | Source | `ticket:references:tickets` |
|---|---|---|
| table visée | `--fields` | `tickets` |
| méthode de filtre du client | `crate::client::ts::nom_de_methode("<table>_filter")` | `ticketsFilter` |
| colonne libellé | `label=<colonne>`, sinon la première colonne textuelle (`string` ou `text`, hors énumération et référence) du `model.rs` de la cible | `sujet` ; `email` pour `users` |
| en-tête de colonne | nom de la relation, humanisé | « Ticket », et non « Ticket id » |

La colonne libellé se lit par un relevé textuel du `model.rs` de la cible, le même que
`generate migration --add-column` pratique déjà (`crates/rbs-cli/src/generate/alter.rs`).

**Grammaire.** `label=<colonne>` rejoint `max=<n>` parmi les modificateurs à valeur
(`crates/rbs-cli/src/generate/fields.rs:656`). Il est **refusé au plan**, avant écriture :

- sur un champ qui n'est pas une référence — comme `cascade` sur un scalaire ;
- quand la cible n'a pas cette colonne, ou qu'elle n'est pas textuelle — le refus nomme les
  colonnes textuelles de la cible ;
- vide (`label=`), ou répété.

**Replis, jamais des refus** — le plan les annonce chacun d'une ligne :

- la cible n'a pas de route de filtre (`operation_id = "<table>_filter"` absent de `src/`) :
  table interne d'`auth`, feature vide de `generate feature`, ou projet dont `auth` précède
  cette version. Le champ reste le texte actuel. Pour `users`, la ligne renvoie à la section
  « Lister les comptes » du guide `auth` — `rbs upgrade` ne touche pas aux contrôleurs
  (`crates/rbs-cli/src/upgrade.rs:3-11`) et ne peut pas poser la route ;
- la cible n'a aucune colonne textuelle et `label=` manque : sélecteur et table affichent
  l'identifiant raccourci (`01a0ce7c…`).

`label=` n'a d'effet que sur l'écran ; sur un projet sans `frontend-admin`, il reste sans
effet, mais ses refus s'appliquent quand même — une colonne mal orthographiée ne doit pas
attendre, silencieuse, le jour où l'écran existera. Le DTO, le modèle, la migration et le
contrat OpenAPI de la table engendrée sont inchangés.

## 2. L'écran

**Deux fichiers génériques**, sous `frontend/src/admin/references/`, un pour tout le projet :

- `ChoixReference.vue` — un champ de saisie qui ouvre une liste de résultats. Props :
  `chercher(motif: string) => Promise<{ cle: string; libelle: string }[]>`, `modelValue`
  (l'identifiant, en chaîne), `libelle` (celui de la valeur courante), `optionnel`. Attente
  de 250 ms après la frappe, 20 résultats au plus, flèches / Entrée / Échap, entrée « aucun »
  quand `optionnel`. Construit sur les composants vendus (`input`, et la liste en éléments
  natifs stylés par le thème) : aucun composant nouveau à vendre.
- `libelles.ts` — `resoudre(chercher, ids)` rend une `Map<id, libelle>` : dédoublonne, un seul
  appel `{ id: { in: [...] } }` avec `per_page` = nombre d'identifiants (100 au plus, une page
  de table n'en porte que 20), cache pour la vie de l'écran.

`frontend-admin` les dépose à l'installation. `generate crud` les dépose s'ils manquent —
projet installé avant cette version — et ne les écrase jamais ; ils paraissent au plan en
`créé`.

**Dans le patron** (`templates/features/frontend-admin/client/src/admin/vues/Patron.vue.jinja`),
pour les seuls champs référence, derrière des blocs conditionnels :

- **table et détail** — un rendu `reference`. Après chaque page chargée, un appel de
  résolution par colonne référence ; la cellule montre le libellé, sinon — en attente ou en
  échec — l'identifiant raccourci, l'UUID complet en `title`. Un échec n'arrête jamais la
  table et n'émet pas de toast ;
- **tri** — une colonne référence n'est pas triable : trier des UUID ne veut rien dire, et
  trier par libellé demanderait la jointure que l'approche écarte ;
- **formulaire** — `ChoixReference`, avec
  `chercher = motif => api.<cible>Filter({ <libelle>: { contains: motif } }, 1, 20)`. En
  modification, il reçoit le libellé déjà résolu pour la table.

Inchangés : le filtre de la liste, la pagination, `Formulaire` (l'identifiant y reste une
chaîne), `corps()`. L'écran de démonstration n'a pas de référence et sort octet pour octet
identique : une template, deux producteurs (ADR-0003).

## 3. La route des comptes

```
POST /users/filter?page=&per_page=       operation_id "users_filter", tag "users"
corps    UserFilter { id?: Comparison<Uuid>, email?: TextMatch, sort? }
réponse  Page<UserSummary { id, email }>
401 sans jeton · 403 en deçà d'admin · 400 filtre illisible
```

- `UserSummary` porte `id` et `email`, rien d'autre ; `UserResponse` (`/auth/me`) inchangé.
- Seuil `require_role(Role::Admin)` : lister les adresses est une donnée personnelle, et
  l'espace d'administration est ouvert à toute session (`frontend/src/admin/garde.ts`).
- `per_page` plafonné à 100, comme partout ; `email contains` porte sur l'adresse
  normalisée.
- Dans les fichiers existants du fragment, couches respectées : `controller/account.rs`
  (handler et `#[utoipa::path]`), `service/account.rs`, `repository/user.rs` (le moteur de
  filtre de `rbs-core`), montage dans `auth/mod.rs`, inscription dans `openapi.rs`. Ni ancre
  ni fragment nouveaux.
- Un compte `user` dans l'espace : `resoudre` reçoit 403 et garde les identifiants
  raccourcis ; le sélecteur affiche « réservé aux administrateurs » à la place des
  résultats.

## 4. Conséquences

**Exemples**, régénérés par diff entre deux générations — jamais par écrasement :

- `blog-auth`, `event-hub` : la route `/users/filter` paraît dans `auth` ;
- `admin-console` : les deux fichiers génériques, une opération de plus au client ;
- `help-desk` : sélecteur de ticket dans `Commentaires.vue`, auteur lu par son email. Les
  retouches Vue du tutoriel (plus de champ auteur au formulaire) sont réappliquées sur les
  écrans neufs ; `the_hand_edits_of_help_desk_are_in_place` en répond toujours.

**Tutoriel** (`tutorials/help-desk.md`, fr et en) : la « limite assumée » de l'étape 5
disparaît au profit du sélecteur ; la commande des commentaires porte
`ticket:references:tickets:cascade:label=sujet` — redondant avec la déduction, écrit pour que
le lecteur voie la syntaxe ; transcriptions recollées depuis les sorties réelles ; « Verify »
cite l'email et le sélecteur.

**Documentation**, fr et en : `cli/generate.md` (grammaire et tableau des modificateurs),
`guides/relations.md` (libellé, `label=`, replis), `guides/frontend.md` (sélecteur,
résolution, compte non admin), `guides/auth.md` (la route, ses droits, et « Lister les
comptes » pour un projet antérieur). `CHANGELOG.md`, section `[Unreleased]`.

## 5. Vérifications

- Unitaires : `label=` accepté et ses quatre refus ; déduction de la colonne libellé ;
  détection de `<table>_filter` ; rendu du patron avec référence, avec chacun des deux replis,
  et sans référence ; écran de démonstration inchangé ; route `users` rendue par la template
  `auth`.
- `cargo test -p rbs-cli --lib`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo fmt --all --check`.
- `integration_examples` (les sept), `integration_docs`.
- Tests de la route dans un projet engendré, contre PostgreSQL : 401, 403, 200, `id in`,
  `email contains`, réponse sans autre champ que `id` et `email`.
- `npm run typecheck && npm run build` sur `admin-console` et `help-desk`.
- `npm run build` et `npm run parite` sous `docs/`.
- Navigateur : l'API et le build se vérifient sans connexion ; le parcours connecté du
  sélecteur est fait par le mainteneur, l'assistant ne saisissant pas de mot de passe.

## Git

Branche `feat/references-dans-les-ecrans`. Commits dans l'ordre : grammaire et déduction ;
route `users` ; fichiers génériques et patron ; exemples ; documentation et tutoriel. Un seul
push, à la fin.
