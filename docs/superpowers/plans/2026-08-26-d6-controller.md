# Génération du controller — plan

**But :** produire `features/<nom>/controller.rs` et le `mod.rs` qui porte `routes()`,
soit les cinq routes HTTP de la feature, documentées.

**Approche :** templates `controller.rs.jinja` et `mod.rs.jinja`, module
`generate/controller.rs`.

```
GET    /<module>        200 Page<<Entite>Response>   extracteur Pagination
POST   /<module>        201 <Entite>Response         ValidatedJson<Create<Entite>>
GET    /<module>/{id}   200 <Entite>Response
PUT    /<module>/{id}   200 <Entite>Response         ValidatedJson<Update<Entite>>
DELETE /<module>/{id}   204
```

**Frontière :** ne connaît que `service.rs`. Les handlers extraient `State<AppState>`, en
tirent `state.core().db()`, et n'écrivent pas une ligne de SeaORM.

**`PUT` et non `PATCH`** alors que la sémantique est celle d'une fusion partielle : c'est
ce qu'on attend d'un scaffold CRUD. Le commentaire du handler dit qu'un champ absent est
conservé — c'est le seul point où le code généré commente autre chose qu'un point
d'extension, et il l'assume.

**`routes()` vit dans `mod.rs`**, conformément à la spec §3.4, et non dans le controller.

## Étapes

- [ ] Test de rendu rouge : cinq handlers, cinq `#[utoipa::path]` avec leur `body`, le
      `routes()` du `mod.rs`, absence de `sea_orm`.
- [ ] `templates/feature/controller.rs.jinja` et `mod.rs.jinja`.
- [ ] `generate/controller.rs` : `rendre(&Feature)` et `rendre_mod(&Feature)`.
- [ ] Étendre `banc.rs` : `mod.rs` fourni par l'appelant, remplissage des ancres
      `<rbs:routes>` et `<rbs:openapi>`, pose d'un test d'intégration dans la crate du
      projet. Le moteur d'ancres reste D9 ; le banc s'en passe à la main, comme il le fait
      déjà pour `<rbs:features>`.
- [ ] Test `#[ignore]` de bout en bout : les six fichiers posés, projet compilé, document
      OpenAPI sérialisé, les cinq chemins et leurs schémas vérifiés.

## Preuve attendue

- ✓ *Les cinq routes apparaissent dans Swagger UI avec leurs schémas* — le test ci-dessus
  prouve que le document les porte ; que Swagger UI les affiche est un critère visuel,
  validé par le porteur du projet sur un `/docs` ouvert. Sans cette validation, la case
  reste `- [ ]` avec une annotation PARTIEL.
