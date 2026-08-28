# W2 — `#[non_exhaustive]` sur ce qui grossira

**Conception.** La seule rupture que ce jalon doit commettre, et elle n'est gratuite
qu'aujourd'hui : après la 1.0.0, elle vaudrait une 2.0.0.

**Le catalogue a été établi sur mesures et validé.** Il ne se rouvre pas. L'inventaire réel
diffère de celui de la conception, qui annonçait 5 enums et 16 structs : `rbs-core` porte
**7 enums** — `LogError` et `LogFormat` s'ajoutent aux cinq nommés — et **18 structs**.

**22 items reçoivent l'attribut :**

- les 7 enums : `Error`, `ConfigError`, `JwtError`, `LogError`, `Status`, `Check`,
  `LogFormat` ;
- les 5 configurations : `Config`, `DocsConfig`, `ServerConfig`, `DatabaseConfig`,
  `AuthConfig` ;
- `ProblemDetails`, `Health`, `Checks`, `CoreState`, `ConnectError`, `Identity`,
  `Pagination`, `Page<T>`, `JsonFormat`, `PrettyFormat`.

**3 en sont exclus, chacun sur une preuve dans le code engendré :**

| Exclu | Preuve |
|---|---|
| `Claims` | `let claims = Claims { … }` — `templates/features/auth/service.rs.jinja:124`. Seul type de `rbs-core` que le code engendré construit littéralement |
| `ValidatedJson<T>` | `ValidatedJson(input): ValidatedJson<…>` dans tous les contrôleurs. L'attribut interdit aussi le pattern matching : chaque handler tomberait |
| `CommonResponses` | `modifiers(&CommonResponses)` — `templates/project/src/openapi.rs.jinja:11`. Struct unité, dont l'attribut interdirait l'instanciation |

**`Page<T>` reçoit l'attribut, contre ce que la conception laissait attendre.** Elle le
citait comme cas dur — « que le code engendré construit à chaque liste » ; la mesure dit
l'inverse : il passe par `Page::new(…)`, jamais par un littéral. Le coût est donc nul.

**Le second critère ne devrait rien coûter, et c'est vérifiable d'avance.** Aucun filtrage
exhaustif d'un enum de `rbs_core` n'existe dans le code engendré : il *construit* des
variantes — `Error::Unauthorized`, `Error::Conflict(…)` —, ce que l'attribut continue
d'autoriser. La rupture ne touche que l'utilisateur qui aurait écrit son propre `match`.

**Le troisième critère est le seul qui atteste que l'attribut sert à quelque chose.** Une
variante ajoutée à `Error` **après** la pose ne doit plus être signalée comme rupture par
`semver-checks`. Il se prouve en deux temps, et les deux comptent : avant la pose, l'ajout
d'une variante doit être signalé — sans quoi le « après » ne démontre rien.

## Étapes

1. Mesurer l'état de départ : ajouter une variante à `Error` **sans** l'attribut, lancer
   `cargo semver-checks --package rbs-core --all-features`, lire le verdict. Il doit
   signaler une rupture. Retirer la variante.
2. Poser l'attribut sur les 22 items du catalogue. Ne pas y ajouter, ne pas en retirer.
3. `cargo build` et `cargo clippy` sur les quatre projets d'`examples/`, **sans modifier une
   ligne** de leur source. Un échec ici est un item mal classé, non un exemple à corriger.
4. Rejouer l'étape 1 avec l'attribut posé : la même variante ajoutée ne doit plus être
   signalée. Retirer la variante.
5. Preuves : les deux verdicts de `semver-checks` encadrant la pose, cités mot pour mot ;
   la compilation des quatre exemples ; le catalogue, item par item, dans le message de
   commit.
