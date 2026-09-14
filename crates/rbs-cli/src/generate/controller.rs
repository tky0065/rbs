//! Rendu de `<name>/controller.rs` et du `mod.rs` qui monte ses routes.

use minijinja::{Value, context};

use crate::template::Renderer;

use super::feature::Feature;

const CONTROLLER: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/templates/feature/controller.rs.jinja"
));

const MODULE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/templates/feature/mod.rs.jinja"
));

/// Rend les handlers de `feature` et leurs annotations OpenAPI.
pub(crate) fn render(feature: &Feature) -> Result<String, minijinja::Error> {
    Renderer::new().render(CONTROLLER, feature)
}

/// Rend le `mod.rs` de `feature` : ses fichiers et son `routes()`.
///
/// `with_tests` déclare le module `tests` : une feature écrite à la main n'en porte pas,
/// et le déclarer empêcherait la compilation.
pub(crate) fn render_mod(feature: &Feature, with_tests: bool) -> Result<String, minijinja::Error> {
    Renderer::new().render(
        MODULE,
        context! { with_tests, ..Value::from_serialize(feature) },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::feature::Feature;
    use crate::generate::{bench, fields};

    fn controller(name: &str) -> String {
        let fields = fields::parse("title:string").expect("champs valides");
        render(&Feature::fresh(name, fields)).expect("le controller doit se rendre")
    }

    fn authenticated(name: &str) -> String {
        let fields = fields::parse("title:string").expect("champs valides");
        render(&Feature::fresh(name, fields).authenticated()).expect("le controller doit se rendre")
    }

    fn authenticated_and_guarded(name: &str, role: &str) -> String {
        let fields = fields::parse("title:string").expect("champs valides");
        render(&Feature::fresh(name, fields).authenticated().guarded(role))
            .expect("le controller doit se rendre")
    }

    fn module(name: &str) -> String {
        let fields = fields::parse("title:string").expect("champs valides");
        render_mod(&Feature::fresh(name, fields), false).expect("le mod.rs doit se rendre")
    }

    fn module_with_tests(name: &str) -> String {
        let fields = fields::parse("title:string").expect("champs valides");
        render_mod(&Feature::fresh(name, fields), true).expect("le mod.rs doit se rendre")
    }

    /// Rend le contrôleur d'une feature dotée de ses routes de contenu.
    fn controller_with_upload(name: &str, fields: &str) -> String {
        let fields = fields::parse(fields).expect("les champs du test doivent être valides");
        render(&Feature::fresh(name, fields).uploading()).expect("le contrôleur doit se rendre")
    }

    fn controller_uploading(name: &str) -> String {
        controller_with_upload(name, "title:string")
    }

    /// Rend le contrôleur d'une feature dont le contenu survit à une suppression logique.
    fn uploading_and_soft_deleting(name: &str) -> String {
        let fields = fields::parse("title:string").expect("champs valides");
        render(&Feature::fresh(name, fields).uploading().soft_deleting())
            .expect("le contrôleur doit se rendre")
    }

    /// Rend le contrôleur d'une feature `auth`, aux deux drapeaux, dotée de ses routes de
    /// contenu.
    fn authenticated_and_guarded_uploading(name: &str, role: &str) -> String {
        let fields = fields::parse("title:string").expect("champs valides");
        render(
            &Feature::fresh(name, fields)
                .authenticated()
                .guarded(role)
                .uploading(),
        )
        .expect("le contrôleur doit se rendre")
    }

    fn module_uploading(name: &str) -> String {
        let fields = fields::parse("title:string").expect("champs valides");
        render_mod(&Feature::fresh(name, fields).uploading(), true)
            .expect("le mod.rs doit se rendre")
    }

    /// Deux formes de ce fichier suivent le nom de l'entité et bornent le point fixe des
    /// deux côtés : la signature de `find`, que rustfmt ramène sur une ligne tant qu'elle
    /// tient sous les cent colonnes de `max_width` — soit jusqu'à deux caractères
    /// d'entité — et l'import des DTO, dont la ligne intérieure déborde à vingt-quatre.
    ///
    /// Au-delà de vingt-trois, rustfmt répartit les trois DTO par remplissage glouton, un
    /// régime que le gabarit ne sait pas écrire et qu'on ne réimplante pas : une montée de
    /// rustfmt le déplacerait. `format::format_batch` le rattrape à l'écriture, donc rien
    /// de mal formé n'atteint l'utilisateur. C'est cette frontière que l'intervalle fixe,
    /// mesurée et non commentée.
    #[test]
    fn the_render_is_already_what_rustfmt_would_write() {
        let divergentes = bench::longueurs_divergentes(controller);

        assert_eq!(
            divergentes,
            (24..=40).collect::<Vec<usize>>(),
            "la plage où le contrôleur diverge de rustfmt a bougé"
        );
    }

    /// Même garde que ci-dessus, pour les trois handlers de contenu qu'ajoute
    /// `--with-upload` : leurs chemins portent eux aussi le nom du module.
    #[test]
    fn the_uploading_controller_render_is_already_what_rustfmt_would_write() {
        let divergentes = bench::longueurs_divergentes(controller_uploading);

        assert_eq!(
            divergentes,
            (24..=40).collect::<Vec<usize>>(),
            "la plage où le contrôleur de contenu diverge de rustfmt a bougé"
        );
    }

    /// La route de collection est le seul appel de ce fichier dont les arguments suivent le
    /// nom du module : ils valent cinquante caractères de plus que lui, et franchissent
    /// donc les soixante colonnes de `fn_call_width` à onze caractères de module.
    ///
    /// Les deux autres routes sont déjà écrites éclatées ou tiennent sans lui.
    #[test]
    fn the_module_render_is_already_what_rustfmt_would_write() {
        let divergentes = bench::longueurs_divergentes(module_with_tests);

        assert_eq!(
            divergentes,
            Vec::<usize>::new(),
            "le rendu de `mod.rs` diverge de rustfmt à ces longueurs de nom"
        );
    }

    /// La route de contenu que `--with-upload` ajoute est déjà éclatée sans condition de
    /// longueur ; ce garde vérifie que ce choix reste un point fixe à toute taille de nom.
    #[test]
    fn the_uploading_module_render_is_already_what_rustfmt_would_write() {
        let divergentes = bench::longueurs_divergentes(module_uploading);

        assert_eq!(
            divergentes,
            Vec::<usize>::new(),
            "le rendu de `mod.rs` avec `--with-upload` diverge de rustfmt à ces longueurs de nom"
        );
    }

    /// `per_page=abc` rend 400 : un document qui ne l'annonce pas fait débugger au client
    /// une pagination qui « ne marche pas », sans rien pour l'aider.
    #[test]
    fn the_list_declares_the_400_of_the_pagination() {
        let rendered = controller("articles");

        let liste = rendered
            .split("pub async fn list(")
            .next()
            .expect("l'annotation précède le handler");

        assert!(
            liste.contains(r#"(status = 400, description = "pagination illisible""#),
            "le 400 de la pagination n'est pas déclaré :\n{liste}"
        );
    }

    /// Le service fusionne : un champ absent du corps garde sa valeur. `PUT` promettrait
    /// un remplacement que ce code ne fait pas ; `PATCH` dit exactement ce qu'il fait.
    #[test]
    fn the_update_is_a_patch_and_no_put_survives() {
        let rendered = controller("articles");
        let module = module("articles");

        assert!(rendered.contains("    patch,"), "{rendered}");
        assert!(!rendered.contains("    put,"), "{rendered}");
        assert!(
            !module.contains(".put("),
            "aucun alias `put` ne survit :\n{module}"
        );
    }

    #[test]
    fn the_five_handlers_are_declared() {
        let rendered = controller("articles");

        for signature in [
            "pub async fn list(",
            "pub async fn create(",
            "pub async fn find(",
            "pub async fn update(",
            "pub async fn delete(",
        ] {
            assert!(
                rendered.contains(signature),
                "« {signature} » absent :\n{rendered}"
            );
        }
    }

    #[test]
    fn each_handler_carries_its_utoipa_annotation() {
        let rendered = controller("articles");

        assert_eq!(
            rendered.matches("#[utoipa::path(").count(),
            6,
            "les six handlers doivent être documentés :\n{rendered}"
        );
    }

    // utoipa retombe sur le nom nu du handler faute d'`operation_id` : deux features CRUD
    // dans un même projet produiraient alors deux opérations d'identifiant `list`, ce que
    // la spécification OpenAPI interdit.
    #[test]
    fn each_handler_carries_an_operation_id_prefixed_by_its_module() {
        let rendered = controller("articles");

        for action in ["list", "create", "find", "update", "delete"] {
            assert!(
                rendered.contains(&format!("operation_id = \"articles_{action}\"")),
                "`{action}` doit porter son operation_id :\n{rendered}"
            );
        }
    }

    #[test]
    fn the_five_verbs_and_their_paths_are_documented() {
        let rendered = controller("blog_posts");

        for annotation in [
            "    get,\n    path = \"/blog_posts\",",
            "    post,\n    path = \"/blog_posts\",",
            "    get,\n    path = \"/blog_posts/{id}\",",
            "    patch,\n    path = \"/blog_posts/{id}\",",
            "    delete,\n    path = \"/blog_posts/{id}\",",
        ] {
            assert!(
                rendered.contains(annotation),
                "annotation attendue absente :\n{annotation}\n---\n{rendered}"
            );
        }
    }

    #[test]
    fn the_response_bodies_name_the_dtos() {
        let rendered = controller("articles");

        assert!(
            rendered.contains("body = Page<ArticleResponse>"),
            "la liste doit annoncer une page :\n{rendered}"
        );
        assert_eq!(
            rendered.matches("body = ArticleResponse").count(),
            3,
            "create, find et update rendent l'entité :\n{rendered}"
        );
        assert!(
            rendered.contains("request_body = CreateArticle")
                && rendered.contains("request_body = UpdateArticle"),
            "les corps de requête doivent être documentés :\n{rendered}"
        );
    }

    #[test]
    fn creation_answers_201_and_deletion_204() {
        let rendered = controller("articles");

        assert!(rendered.contains("status = 201"), "{rendered}");
        assert!(rendered.contains("status = 204"), "{rendered}");
        assert!(
            rendered.contains("Ok((StatusCode::CREATED, Json(article)))"),
            "la création doit rendre 201 :\n{rendered}"
        );
        assert!(
            rendered.contains("Ok(StatusCode::NO_CONTENT)"),
            "la suppression doit rendre 204 :\n{rendered}"
        );
    }

    #[test]
    fn absence_is_documented_where_it_can_occur() {
        let rendered = controller("articles");

        assert_eq!(
            rendered.matches("status = 404").count(),
            3,
            "find, update et delete peuvent ne rien trouver :\n{rendered}"
        );
    }

    #[test]
    fn the_conflict_the_repository_can_raise_is_documented() {
        let rendered = controller("articles");

        assert_eq!(
            rendered.matches("status = 409").count(),
            2,
            "create et update traduisent un doublon en conflit, le contrat doit le \
             dire :\n{rendered}"
        );
    }

    #[test]
    fn incoming_bodies_go_through_the_core_validation() {
        let rendered = controller("articles");

        assert!(
            rendered.contains("ValidatedJson(input): ValidatedJson<CreateArticle>"),
            "la création doit valider son corps :\n{rendered}"
        );
        assert!(
            rendered.contains("ValidatedJson(input): ValidatedJson<UpdateArticle>"),
            "la mise à jour doit valider son corps :\n{rendered}"
        );
    }

    #[test]
    fn no_seaorm_query_reaches_the_http_layer() {
        let rendered = controller("articles");

        assert!(
            !rendered.contains("sea_orm::Entity") && !rendered.contains("ActiveModel"),
            "le controller ne connaît que service.rs :\n{rendered}"
        );
        assert!(
            !rendered.contains("super::repository") && !rendered.contains("super::model"),
            "le controller ne connaît que service.rs :\n{rendered}"
        );
    }

    /// Le bloc d'un handler : son annotation et sa fonction, isolées du reste du fichier.
    ///
    /// Un compte global sur le fichier entier ne verrait pas un échange apparié — `filter`
    /// portant le seuil de `create`, et réciproquement — c'est ce que nommer chaque route
    /// permet d'affirmer que les comptes seuls ne prouvent pas.
    fn handler<'a>(rendered: &'a str, name: &str) -> &'a str {
        rendered
            .split("#[utoipa::path(")
            .find(|bloc| bloc.contains(&format!("pub async fn {name}(")))
            .unwrap_or_else(|| panic!("handler `{name}` absent :\n{rendered}"))
    }

    /// La carte route → rôle sous `auth` et `--role`, nommée route par route : les quatre
    /// écritures montent au rôle demandé, les cinq lectures restent au seuil par défaut.
    #[test]
    fn under_a_role_each_route_names_its_own_threshold() {
        let rendered = authenticated_and_guarded_uploading("articles", "admin");

        for (name, role) in [
            ("create", "Admin"),
            ("update", "Admin"),
            ("delete", "Admin"),
            ("put_content", "Admin"),
            ("list", "User"),
            ("filter", "User"),
            ("find", "User"),
            ("get_content", "User"),
            ("head_content", "User"),
        ] {
            let bloc = handler(&rendered, name);

            assert!(
                bloc.contains(&format!("identite.require_role(Role::{role})?;")),
                "`{name}` doit porter le rôle `{role}` :\n{bloc}"
            );
        }
    }

    /// `Identity` implémente `FromRequestParts` : `identite` doit précéder, dans la
    /// signature, tout extracteur qui consomme le corps de la requête.
    #[test]
    fn under_auth_identite_precedes_every_body_consuming_extractor() {
        let rendered = authenticated_and_guarded_uploading("articles", "admin");

        for (name, extracteur_de_corps) in [
            ("filter", "Json(filtre)"),
            ("create", "ValidatedJson(input)"),
            ("update", "ValidatedJson(input)"),
            ("put_content", "content: Bytes"),
        ] {
            let bloc = handler(&rendered, name);
            let position_identite = bloc
                .find("identite: Identity,")
                .unwrap_or_else(|| panic!("`{name}` doit porter `identite` :\n{bloc}"));
            let position_corps = bloc.find(extracteur_de_corps).unwrap_or_else(|| {
                panic!("`{name}` doit porter `{extracteur_de_corps}` :\n{bloc}")
            });

            assert!(
                position_identite < position_corps,
                "`identite` doit précéder `{extracteur_de_corps}` dans `{name}` :\n{bloc}"
            );
        }
    }

    /// Les six routes portent la même garde, lectures comprises.
    #[test]
    fn under_auth_every_route_requires_a_token() {
        let rendered = authenticated("articles");

        assert_eq!(
            rendered.matches("identite: Identity,").count(),
            6,
            "les six routes doivent extraire l'identité :\n{rendered}"
        );
        assert_eq!(
            rendered
                .matches("identite.require_role(Role::User)?;")
                .count(),
            6,
            "les six routes doivent porter le seuil par défaut :\n{rendered}"
        );
        assert_eq!(
            rendered.matches(r#"security(("bearer" = [])),"#).count(),
            6,
            "les six annotations doivent déclarer le schéma :\n{rendered}"
        );
        assert_eq!(
            rendered.matches("status = 401").count(),
            6,
            "les six annotations doivent documenter le refus sans jeton :\n{rendered}"
        );
        assert_eq!(
            rendered.matches("status = 403").count(),
            6,
            "les six annotations doivent documenter le rôle insuffisant :\n{rendered}"
        );
    }

    /// La garde est une préface au corps, et s'en détache partout de la même façon.
    ///
    /// `rustfmt` n'ajoute pas de ligne vide : un blanc mangé par un `{%- else %}` ne se
    /// voit qu'ici, ou à l'œil dans un exemple.
    #[test]
    fn under_auth_every_guard_is_followed_by_a_blank_line() {
        let rendered = authenticated("articles");

        assert_eq!(
            rendered
                .matches("identite.require_role(Role::User)?;\n\n")
                .count(),
            6,
            "chaque garde doit être suivie d'une ligne vide :\n{rendered}"
        );
    }

    /// `--role` ne substitue le nom que sur les écritures.
    #[test]
    fn a_role_raises_the_threshold_of_the_writes_only() {
        let rendered = authenticated_and_guarded("articles", "admin");

        assert_eq!(
            rendered
                .matches("identite.require_role(Role::Admin)?;")
                .count(),
            3,
            "create, update et delete doivent monter au rôle demandé :\n{rendered}"
        );
        assert_eq!(
            rendered
                .matches("identite.require_role(Role::User)?;")
                .count(),
            3,
            "list, filter et find restent au seuil par défaut :\n{rendered}"
        );
    }

    #[test]
    fn the_guard_names_the_role_in_pascal_case() {
        let rendered = authenticated_and_guarded("articles", "super_admin");

        assert!(
            rendered.contains("identite.require_role(Role::SuperAdmin)?;"),
            "le rôle doit se traduire en variante de l'enum :\n{rendered}"
        );
    }

    /// Le mode d'emploi est écrit une fois, en tête du fichier.
    #[test]
    fn the_way_to_open_a_route_is_documented_once() {
        let rendered = authenticated("articles");

        assert_eq!(
            rendered.matches("retirez le paramètre").count(),
            1,
            "le mode d'emploi ne se répète pas sur chaque handler :\n{rendered}"
        );
        assert!(
            rendered.starts_with("//!"),
            "le bandeau ouvre le fichier :\n{rendered}"
        );
    }

    /// Témoin : sans `auth`, le rendu ne porte rien du garde.
    #[test]
    fn without_auth_the_controller_carries_nothing_of_the_guard() {
        let rendered = controller("articles");

        assert!(
            !rendered.contains("Identity")
                && !rendered.contains("require_role")
                && !rendered.contains("bearer"),
            "sans `auth`, le rendu est inchangé :\n{rendered}"
        );
    }

    /// Le garde allonge trois signatures, et celle de `delete` franchit les cent colonnes
    /// où rustfmt bascule : la template doit l'écrire déjà éclatée.
    ///
    /// La frontière haute est la même que sans garde — l'import des DTO, à vingt-quatre
    /// caractères d'entité. La basse en diffère : `Identity` allonge la signature de
    /// `find`, qui ne se compacte donc jamais.
    #[test]
    fn the_guarded_render_is_already_what_rustfmt_would_write() {
        let divergentes =
            bench::longueurs_divergentes(|name| authenticated_and_guarded(name, "admin"));

        assert_eq!(
            divergentes,
            (24..=40).collect::<Vec<usize>>(),
            "la plage où le contrôleur gardé diverge de rustfmt a bougé"
        );
    }

    /// La ligne des DTO est la seule du controller qui grandisse avec le nom de l'entité.
    /// Ce que la comparaison à rustfmt prouve globalement, ce test le nomme : sous le
    /// seuil la ligne reste entière, au-dessus la template l'éclate elle-même.
    #[test]
    fn the_dto_import_splits_itself_once_a_single_line_would_overflow() {
        let court = controller("articles");
        let long = controller("administrative_documents");

        assert!(
            court.contains("use super::dto::{ArticleResponse, CreateArticle, UpdateArticle};\n"),
            "un nom court tient sur une ligne :\n{court}"
        );
        assert!(
            long.contains(
                "use super::dto::{\n    AdministrativeDocumentResponse, \
                 CreateAdministrativeDocument, UpdateAdministrativeDocument,\n};\n"
            ),
            "un nom long doit être rendu déjà éclaté :\n{long}"
        );
    }

    #[test]
    fn the_module_mounts_the_five_routes() {
        let rendered = module("articles");

        assert!(
            rendered
                .contains(".route(\"/articles\", get(controller::list).post(controller::create))"),
            "routes de collection absentes :\n{rendered}"
        );
        assert!(
            rendered.contains("\"/articles/{id}\"")
                && rendered.contains("get(controller::find)")
                && rendered.contains(".patch(controller::update)")
                && rendered.contains(".delete(controller::delete)"),
            "routes unitaires absentes :\n{rendered}"
        );
    }

    /// La route littérale se monte avant `/{id}`, sans quoi `filter` serait lu comme un
    /// identifiant — c'est ce que fait déjà `broadcast` dans `examples/newsletter-queue`.
    #[test]
    fn the_filter_route_is_mounted_before_the_id_route() {
        let rendered = module("articles");

        // Les chemins sont cherchés entre guillemets : le commentaire qui précède la
        // route nomme `/articles/{id}` sans les siens, et serait trouvé le premier.
        let filtre = rendered
            .find(r#""/articles/filter""#)
            .expect("route de filtre montée");
        let id = rendered
            .find(r#""/articles/{id}""#)
            .expect("route d'identifiant montée");

        assert!(
            filtre < id,
            "`filter` doit précéder l'identifiant :\n{rendered}"
        );
    }

    #[test]
    fn the_module_declares_the_six_files_of_the_feature() {
        let rendered = module("articles");

        for declaration in [
            "pub mod controller;",
            "pub mod dto;",
            "pub mod model;",
            "pub mod repository;",
            "pub mod service;",
        ] {
            assert!(
                rendered.contains(declaration),
                "« {declaration} » absent :\n{rendered}"
            );
        }
    }

    #[test]
    fn the_module_declares_the_tests_when_they_are_generated() {
        let avec = module_with_tests("articles");

        assert!(
            avec.contains("#[cfg(test)]\nmod tests;"),
            "le module de tests doit être déclaré :\n{avec}"
        );
    }

    /// Une feature écrite à la main n'a pas de `tests.rs` : le déclarer empêcherait la
    /// compilation du projet.
    #[test]
    fn the_module_declares_no_tests_when_there_are_none() {
        let sans = module("articles");

        assert!(!sans.contains("mod tests;"), "{sans}");
    }

    #[test]
    fn the_three_content_handlers_are_rendered() {
        let rendered = controller_with_upload("articles", "title:string");

        for handler in ["put_content", "get_content", "head_content"] {
            assert!(
                rendered.contains(&format!("pub async fn {handler}")),
                "{handler} manque :\n{rendered}"
            );
        }
        assert!(
            rendered.contains("content: Bytes"),
            "le corps voyage brut : en JSON il passerait en base64, donc deux fois en \
             mémoire :\n{rendered}"
        );
    }

    /// Sous `--soft-delete`, le contenu reste : le service ne prend donc plus le magasin
    /// pour supprimer, et le contrôleur ne le lui passe pas.
    #[test]
    fn a_logical_deletion_hands_the_service_no_store() {
        let rendered = uploading_and_soft_deleting("articles");

        assert!(
            rendered.contains("service::delete(state.core().db(), id).await?;"),
            "le magasin n'a rien à faire dans cet appel :\n{rendered}"
        );
    }

    /// Témoin : hors suppression logique, le magasin voyage avec l'appel.
    #[test]
    fn a_hard_deletion_hands_the_service_its_store() {
        let rendered = controller_uploading("articles");

        assert!(
            rendered.contains(
                "service::delete(state.core().db(), state.storage().as_ref(), id).await?;"
            ),
            "le contenu part avec la ligne :\n{rendered}"
        );
    }

    /// Les trois routes de contenu suivent la même règle.
    #[test]
    fn under_auth_the_content_routes_require_a_token_too() {
        let fields = fields::parse("title:string").expect("champs valides");
        let rendered = render(
            &Feature::fresh("uploads", fields)
                .authenticated()
                .uploading(),
        )
        .expect("le controller doit se rendre");

        assert_eq!(
            rendered
                .matches("identite.require_role(Role::User)?;")
                .count(),
            9,
            "les neuf routes doivent porter la garde :\n{rendered}"
        );
    }

    /// `PUT /<ressource>/{id}/content` est une écriture : `--role` la monte comme
    /// `create`, `update` et `delete`. `GET` et `HEAD` restent au seuil par défaut, comme
    /// `list`, `filter` et `find`.
    #[test]
    fn a_role_raises_the_threshold_of_the_content_writes_only() {
        let rendered = authenticated_and_guarded_uploading("articles", "admin");

        assert_eq!(
            rendered
                .matches("identite.require_role(Role::Admin)?;")
                .count(),
            4,
            "create, update, delete et put_content doivent monter au rôle demandé :\n{rendered}"
        );
        assert_eq!(
            rendered
                .matches("identite.require_role(Role::User)?;")
                .count(),
            5,
            "list, filter, find, get_content et head_content restent au seuil par défaut :\n{rendered}"
        );
    }

    /// Neuf routes sous `auth`, donc neuf contrats à annoncer.
    #[test]
    fn under_auth_every_uploading_route_declares_the_bearer_and_the_two_refusals() {
        let fields = fields::parse("title:string").expect("champs valides");
        let rendered = render(
            &Feature::fresh("uploads", fields)
                .authenticated()
                .uploading(),
        )
        .expect("le controller doit se rendre");

        for annotation in [
            r#"security(("bearer" = [])),"#,
            "status = 401",
            "status = 403",
        ] {
            assert_eq!(
                rendered.matches(annotation).count(),
                9,
                "« {annotation} » doit figurer sur les neuf routes :\n{rendered}"
            );
        }
    }

    /// Témoin : sans `auth`, le rendu des routes de contenu ne porte rien du garde.
    #[test]
    fn without_auth_the_content_routes_carry_nothing_of_the_guard() {
        let rendered = controller_uploading("articles");

        assert!(
            !rendered.contains("Identity")
                && !rendered.contains("require_role")
                && !rendered.contains("bearer"),
            "sans `auth`, le rendu des routes de contenu est inchangé :\n{rendered}"
        );
    }

    /// Le garde allonge une quatrième signature, déjà éclatée sans condition de longueur.
    /// La frontière reste celle des autres rendus du contrôleur.
    #[test]
    fn the_guarded_uploading_render_is_already_what_rustfmt_would_write() {
        let divergentes =
            bench::longueurs_divergentes(|name| authenticated_and_guarded_uploading(name, "admin"));

        assert_eq!(
            divergentes,
            (24..=40).collect::<Vec<usize>>(),
            "la plage où le contrôleur gardé à routes de contenu diverge de rustfmt a bougé"
        );
    }

    /// Le rendu entier du contrôleur sous le drapeau, figé octet à octet.
    ///
    /// `examples/file-drop` retouche `src/uploads/controller.rs` à la main : le fichier
    /// sort de la comparaison des exemples, et c'est pourtant celui que le guide du
    /// stockage cite. Les assertions ci-dessus cherchent chacune une chaîne ; aucune ne
    /// verrait une ligne vide perdue entre deux blocs, que rustfmt ne rétablit pas.
    #[test]
    fn the_uploading_controller_renders_the_frozen_fixture() {
        bench::fige(
            "fixtures/uploads/controller.rs",
            &render(&bench::uploads()).expect("le contrôleur doit se rendre"),
        );
    }

    #[test]
    fn the_content_routes_are_mounted() {
        let fields = fields::parse("title:string").expect("champs");
        let rendered = render_mod(&Feature::fresh("articles", fields).uploading(), false)
            .expect("le mod doit se rendre");

        assert!(
            rendered.contains(r#""/articles/{id}/content""#),
            "les trois routes partagent un chemin :\n{rendered}"
        );
        assert!(
            rendered.contains("put(controller::put_content)")
                && rendered.contains("get(controller::get_content)")
                && rendered.contains("head(controller::head_content)"),
            "{rendered}"
        );
    }

    #[test]
    fn the_upload_route_alone_raises_the_body_limit() {
        let fields = fields::parse("title:string").expect("champs");
        let rendered = render_mod(&Feature::fresh("articles", fields).uploading(), false)
            .expect("le mod doit se rendre");

        assert_eq!(
            rendered.matches("DefaultBodyLimit::max").count(),
            1,
            "posée sur le routeur, la limite relèverait aussi celle des routes JSON, \
             qu'aucun besoin ne justifie :\n{rendered}"
        );
        assert!(rendered.contains("const TAILLE_MAX"), "{rendered}");
    }

    #[test]
    fn an_ordinary_module_mounts_no_content_route() {
        let fields = fields::parse("title:string").expect("champs");
        let rendered =
            render_mod(&Feature::fresh("articles", fields), false).expect("le mod doit se rendre");

        assert!(
            !rendered.contains("content") && !rendered.contains("DefaultBodyLimit"),
            "témoin :\n{rendered}"
        );
    }

    /// Rend le contrôleur d'une feature dont la liste `GET` pagine par curseur.
    fn by_cursor(name: &str) -> String {
        let fields = fields::parse("title:string").expect("champs valides");
        render(&Feature::fresh(name, fields).paged_by_cursor())
            .expect("le contrôleur doit se rendre")
    }

    /// Le même, sous `auth`.
    fn by_cursor_authenticated(name: &str) -> String {
        let fields = fields::parse("title:string").expect("champs valides");
        render(
            &Feature::fresh(name, fields)
                .paged_by_cursor()
                .authenticated(),
        )
        .expect("le contrôleur doit se rendre")
    }

    /// Sous `--cursor`, `list` extrait le curseur du noyau et annonce la page qu'il rend :
    /// un document qui promettrait encore `page` ferait chercher au client un paramètre que
    /// la route ignore.
    #[test]
    fn under_cursor_the_list_takes_a_cursor_and_returns_a_cursor_page() {
        let rendered = by_cursor("articles");
        let liste = handler(&rendered, "list");

        for attendu in [
            r#"("after" = Option<Uuid>, Query, description = "identifiant après lequel reprendre ; absent, la première page"),"#,
            r#"("per_page" = Option<u64>, Query, description = "éléments par page, 100 au plus")"#,
            r#"(status = 200, description = "page de articles", body = CursorPage<ArticleResponse>),"#,
            r#"(status = 400, description = "curseur ou pagination illisible", body = ProblemDetails, content_type = "application/problem+json")"#,
            "    cursor: Cursor,\n) -> Result<Json<CursorPage<ArticleResponse>>> {\n    \
             Ok(Json(service::list(state.core().db(), &cursor).await?))\n}",
        ] {
            assert!(
                liste.contains(attendu),
                "« {attendu} » absent de `list` :\n{liste}"
            );
        }
        for absent in [r#"("page" = "#, "pagination: Pagination", "body = Page<"] {
            assert!(
                !liste.contains(absent),
                "« {absent} » survit dans `list` sous --cursor :\n{liste}"
            );
        }
    }

    /// La route de filtre garde sa pagination par page : un curseur sur l'`id` serait faux
    /// dès que le tri, libre, porte sur une autre colonne.
    #[test]
    fn under_cursor_the_filter_keeps_its_page() {
        let rendered = by_cursor("articles");
        let filtre = handler(&rendered, "filter");

        for attendu in [
            r#"("page" = Option<u64>, Query, description = "numéro de page, à partir de 1"),"#,
            r#"body = Page<ArticleResponse>),"#,
            "    pagination: Pagination,\n    Json(filtre): Json<ArticleFilter>,\n\
             ) -> Result<Json<Page<ArticleResponse>>> {",
            "service::filter(state.core().db(), &filtre, &pagination)",
        ] {
            assert!(
                filtre.contains(attendu),
                "« {attendu} » absent de `filter` sous --cursor :\n{filtre}"
            );
        }
    }

    /// Les deux pages servent sous `--cursor` : `Page` et `Pagination` à `filter`, `Cursor`
    /// et `CursorPage` à `list`. Le projet engendré compile sous `-D warnings`, où un
    /// import de trop arrête la construction autant qu'un import manquant.
    #[test]
    fn under_cursor_the_core_import_names_both_pages() {
        for rendered in [by_cursor("articles"), by_cursor_authenticated("articles")] {
            let import = rendered
                .split("use rbs_core::{")
                .nth(1)
                .and_then(|suite| suite.split("};").next())
                .unwrap_or_else(|| panic!("l'import du noyau est rendu :\n{rendered}"));
            let noms: Vec<&str> = import
                .split([',', ' ', '\n'])
                .filter(|nom| !nom.is_empty())
                .collect();

            for nom in ["Cursor", "CursorPage", "Page", "Pagination"] {
                assert!(
                    noms.contains(&nom),
                    "`{nom}` manque à l'import du noyau : {noms:?}"
                );
            }
        }
    }

    /// Sous `auth`, la liste par curseur reste fermée comme les cinq autres routes, et sa
    /// garde se détache du corps comme partout ailleurs.
    #[test]
    fn under_cursor_and_auth_the_list_keeps_its_guard() {
        let rendered = by_cursor_authenticated("articles");
        let liste = handler(&rendered, "list");

        for attendu in [
            r#"security(("bearer" = [])),"#,
            "status = 401",
            "status = 403",
            "    identite: Identity,\n    cursor: Cursor,\n",
            "    identite.require_role(Role::User)?;\n\n    \
             Ok(Json(service::list(state.core().db(), &cursor).await?))",
        ] {
            assert!(
                liste.contains(attendu),
                "« {attendu} » absent de `list` sous --cursor et auth :\n{liste}"
            );
        }
        assert_eq!(
            rendered
                .matches("identite.require_role(Role::User)?;\n\n")
                .count(),
            6,
            "les six routes gardent leur garde, suivie d'une ligne vide :\n{rendered}"
        );
    }

    /// Témoin : sans le drapeau, le contrôleur ne porte rien du curseur.
    #[test]
    fn without_cursor_the_controller_carries_nothing_of_the_cursor() {
        let rendered = controller("articles");

        assert!(
            !rendered.contains("Cursor") && !rendered.contains(r#""after""#),
            "sans `--cursor`, la liste pagine par page :\n{rendered}"
        );
    }

    /// Le handler `list`, seul propre au curseur, isolé du reste du fichier.
    fn cursor_list(source: &str) -> Option<String> {
        source
            .split("pub async fn list(")
            .nth(1)
            .and_then(|suite| suite.split("\n}\n").next())
            .map(str::to_owned)
    }

    /// La même garde sous `--cursor`, et le même ensemble que le rendu par défaut : l'import
    /// des DTO le borne dès vingt-quatre caractères. L'import du noyau, allongé de
    /// `Cursor` et `CursorPage`, ne suit pas le nom de l'entité — il est écrit éclaté une
    /// fois pour toutes.
    ///
    /// Au-delà de vingt-trois, la divergence de l'import masquerait toute autre : le
    /// handler `list` est donc comparé à part à la sortie de rustfmt, à chaque longueur.
    #[test]
    fn the_cursor_render_is_already_what_rustfmt_would_write() {
        assert_eq!(
            bench::longueurs_divergentes(by_cursor),
            (24..=40).collect::<Vec<usize>>(),
            "la plage où le contrôleur diverge de rustfmt a bougé sous --cursor"
        );

        for taille in 1..=40 {
            let rendered = by_cursor(&("a".repeat(taille - 1) + "e"));
            assert_eq!(
                cursor_list(&rendered),
                cursor_list(&bench::formatted(&rendered)),
                "le `list` du curseur s'écarte de rustfmt à {taille} caractères"
            );
        }
    }

    /// La même garde sous `--cursor` et `auth` : `identite` allonge l'import du noyau et la
    /// signature de `list`, sans que ni l'un ni l'autre ne suive le nom de l'entité.
    #[test]
    fn the_guarded_cursor_render_is_already_what_rustfmt_would_write() {
        assert_eq!(
            bench::longueurs_divergentes(by_cursor_authenticated),
            (24..=40).collect::<Vec<usize>>(),
            "la plage où le contrôleur gardé diverge de rustfmt a bougé sous --cursor"
        );

        for taille in 1..=40 {
            let rendered = by_cursor_authenticated(&("a".repeat(taille - 1) + "e"));
            assert_eq!(
                cursor_list(&rendered),
                cursor_list(&bench::formatted(&rendered)),
                "le `list` gardé du curseur s'écarte de rustfmt à {taille} caractères"
            );
        }
    }

    /// Le rendu entier du contrôleur sous `--cursor` et `auth`, figé octet à octet : aucun
    /// exemple n'emploie le drapeau, et rustfmt ne rétablit pas une ligne vide perdue sous
    /// la bascule.
    #[test]
    fn the_cursor_controller_renders_the_frozen_fixture() {
        bench::fige(
            "fixtures/cursor/controller.rs",
            &render(&bench::articles_par_curseur().authenticated())
                .expect("le contrôleur doit se rendre"),
        );
    }

    /// Ce que le projet généré vérifie de son propre document OpenAPI.
    ///
    /// Le projet est un binaire : un test d'intégration ne pourrait pas atteindre son
    /// `ApiDoc`. La vérification est donc posée comme module de test du binaire lui-même.
    const VERIFICATION: &str = r#"use utoipa::OpenApi;

use demo_api::openapi::ApiDoc;

#[test]
fn the_five_routes_of_the_feature_are_documented() {
    let doc = ApiDoc::openapi();

    let collection = doc
        .paths
        .paths
        .get("/articles")
        .expect("chemin de collection absent du document");
    assert!(collection.get.is_some(), "GET de collection absent");
    assert!(collection.post.is_some(), "POST de collection absent");

    let unit = doc
        .paths
        .paths
        .get("/articles/{id}")
        .expect("chemin unitaire absent du document");
    assert!(unit.get.is_some(), "GET unitaire absent");
    assert!(unit.patch.is_some(), "PATCH unitaire absent");
    assert!(unit.delete.is_some(), "DELETE unitaire absent");

    let filtre = doc
        .paths
        .paths
        .get("/articles/filter")
        .expect("la route de filtrage doit etre documentee");
    assert!(filtre.post.is_some(), "POST de filtrage absent");
}

#[test]
fn chaque_route_annonce_le_schema_qu_elle_rend() {
    let doc = ApiDoc::openapi();
    let composants = doc.components.expect("composants absents du document");
    let names: Vec<&str> = composants.schemas.keys().map(String::as_str).collect();

    for expected in [
        "ArticleResponse",
        "CreateArticle",
        "UpdateArticle",
        "ArticleFilter",
    ] {
        assert!(
            names.contains(&expected),
            "schema {expected} absent, present : {names:?}"
        );
    }
    assert!(
        names.iter().any(|name| name.contains("Page")),
        "le schema de la page est absent, present : {names:?}"
    );
}
"#;

    #[test]
    #[ignore = "compile un projet Axum + SeaORM complet : plusieurs minutes"]
    fn the_five_routes_appear_in_the_openapi_document() {
        let fields = "title:string,email:string:unique,summary:text:optional,views:int,\
                      published:bool,auteur_id:uuid,published_at:datetime";
        let fields = fields::parse(fields).expect("champs valides");
        let feature = Feature::fresh("articles", fields);

        let project = bench::Project::fresh();
        project.write_feature("articles", &bench::tous(&feature, false));
        project.mount_feature("articles");
        project.write_unit_test("verification_openapi", VERIFICATION);
        project.test_of();
    }

    /// Monte un projet complet sous `target/workshop/`, pour la revue de Swagger UI.
    #[test]
    #[ignore = "atelier : laisse un projet démarrable derrière lui"]
    fn workshop() {
        let fields = "title:string,email:string:unique,summary:text:optional,views:int,\
                      published:bool,auteur_id:uuid,published_at:datetime";
        let fields = fields::parse(fields).expect("champs valides");
        let feature = Feature::fresh("articles", fields);

        let project = bench::Project::fresh();
        project.write_feature("articles", &bench::tous(&feature, false));
        project.mount_feature("articles");

        println!("{}", project.keep().display());
    }
}
