# Les écrans d'administration engendrés — plan

Issue **#23**, sous la spec **#13**, décision en `docs/adr/0003-ecrans-d-administration-engendres.md`.

## Ce qui est déjà là

Le jalon a livré la moitié amont : `src/ecran.rs` décrit ce que la template attend,
`templates/features/frontend-admin/client/src/admin/vues/Patron.vue.jinja` est l'écran
patron, le fragment le rend pour son entité de démonstration, et les deux ancres
`admin_routes` et `admin_rail` existent. Il manque le second producteur.

## La forme retenue

Un **écran par table**, un seul fichier, portant la liste filtrée, le formulaire et le
détail — c'est la définition du glossaire (« Écran engendré »), et c'est ce qui rend vraie
la promesse d'ADR-0003 : « ce qu'on voit à l'installation est exactement ce qu'on obtiendra
ensuite ». Donc l'écran de démonstration gagne lui aussi son formulaire et son détail : une
seule forme, deux producteurs.

Ce qui dépend d'un contrat entre les deux producteurs tient dans **quatre fonctions de
source** — `interroger`, `lire`, `enregistrer`, `supprimer` — plus `depuis` et `raison`, qui
les accompagnent. Le fragment les rend sur un tableau écrit dans le fichier ; la commande
les rend sur le client typé (`api.<module>Filter`, `Find`, `Create`, `Update`, `Delete`).
Tout le reste de l'écran — filtre, tri, pagination, dialogue, panneau, rendu — ne connaît
que `Ligne`, `Requete` et `Formulaire`.

Une cinquième valeur les sépare, qui ne dépend d'aucun contrat : la **taille de page**,
cinq sur la démonstration — ses onze lignes doivent montrer trois pages — et vingt sur une
table réelle. Elle est nommée des deux côtés dans `src/ecran.rs`.

`Ecran` porte donc trois listes, chacune avec un rôle distinct :

- `proprietes` : la forme d'une ligne, une propriété par clé (`interface Ligne`) ;
- `colonnes` : le sous-ensemble affiché dans la table, et son tri ;
- `champs` : le sous-ensemble éditable, un contrôle par champ (le formulaire).

`api: Option<Api>` décide du régime : `None` est l'écran de démonstration.

## Étapes

- [x] **1. `Ecran` s'ouvre aux deux producteurs.** `Propriete`, `Champ`, `Api` dans
  `src/ecran.rs` ; `Textes` reçoit les libellés du formulaire et du détail ;
  `Ecran::demonstration` reconstruit sur `proprietes`/`champs`. — Fait le 2026-09-19 :
  `cargo test -p rbs-cli --lib ecran`, 9 passés.
- [x] **2. `Ecran::pour`.** Construit l'écran d'une `Feature` : segment, route, composant,
  propriétés (`id`, les champs, `created_at`, `updated_at`), colonnes (les champs puis
  `updated_at`), champs du formulaire avec leur contrôle, et l'`Api` dont les cinq méthodes
  portent le nom que `client::ts` donnera. — Fait le 2026-09-19 :
  `a_generated_screen_mounts_itself_the_same_way`,
  `each_control_of_the_form_is_rendered_for_its_column_type`,
  `an_optional_column_sends_null_when_it_is_left_empty`.
- [x] **3. L'écran patron porte les deux régimes.** Formulaire en `Dialog`, détail en
  `Sheet`, quatre fonctions de source branchées sur `ecran.api`. — Fait le 2026-09-19 :
  `the_two_producers_differ_only_by_their_source` et
  `the_filter_targets_the_first_textual_column_or_disappears`.
- [x] **4. Le montage.** `mount::for_admin_screen(&Ecran)` rend les deux `Mount`. — Fait le
  2026-09-19 : `the_admin_screen_targets_the_two_anchors_of_the_shell`, qui vérifie aussi
  l'appartenance au registre `ANCRES`.
- [x] **5. `--no-admin`.** Le drapeau dans `cli.rs`, `Options.no_admin`, passage dans
  `lib.rs`. — Fait le 2026-09-19 : `generate_crud_accepts_no_admin` et
  `generate_crud_emits_the_screens_by_default`, qui tient aussi qu'aucun `--with-admin`
  n'existe.
- [x] **6. Le plan de `generate crud`.** Le fichier d'écran et les deux insertions quand
  `frontend-admin` est au manifeste et que le drapeau est absent. — Fait le 2026-09-19 :
  les quatre cas dans `generate::command::tests`, plus la balise retirée à la main —
  `cargo test -p rbs-cli --lib generate::command`, 61 passés.
- [x] **7. La même template des deux côtés.** — Fait le 2026-09-19 :
  `the_embedded_pattern_screen_is_the_one_the_fragment_lays_down` refuse désormais toute
  copie sous `templates/feature/` et vérifie que la commande lit le chemin du fragment.
- [x] **8. La chaîne réelle.** — Fait le 2026-09-19 :
  `integration_frontend the_admin_shell_generates_its_client_typechecks_and_builds`
  `--ignored`, 1 passé en 52 s : `npm run typecheck` et `npm run build` verts sur un projet
  portant l'écran engendré d'une table à neuf types de colonnes, et le morceau
  `Bordereaux-` dans `dist/assets`.

## Ce que le plan a trouvé en route

`rbs generate client` rendait tout composant non-objet en `export interface X <union>`, qui
n'est pas du TypeScript : toute entité portant un champ `enum` déclare un tel composant, et
le client entier cessait alors de s'analyser. Corrigé dans `client::ts::declaration_de` —
un alias de type quand le corps n'est pas un objet — parce que l'écran engendré importe le
type du corps de la ressource et que la vérification des types en dépend.

## Ce que le plan ne fait pas

`rbs remove frontend-admin` ne retire pas les écrans engendrés : ce sont des fichiers de
l'utilisateur, comme `src/<module>/`, que le retrait d'un fragment n'a jamais touchés.
