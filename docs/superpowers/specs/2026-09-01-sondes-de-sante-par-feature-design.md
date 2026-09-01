# Sondes de santé par feature installée

**Tâche** : `IMPROVE.md` #55. **Date** : 2026-09-01. **Statut** : validé, prêt à planifier.

## Le problème

`GET /health` répond `ok` alors que Redis ou S3 sont morts. `rbs-core::health` ne connaît
qu'une dépendance — `Checks { database: Check }` (`health.rs:56-59`) — et son handler ne
pingue que la base (`:85`). Un projet qui a installé `redis` et `storage` a trois
dépendances et n'en fait contrôler qu'une : l'orchestrateur garde en rotation un pod dont
le cache est injoignable.

Le commentaire de `Checks` annonce d'ailleurs l'extension depuis le premier jour — « une
dépendance ajoutée plus tard — cache, file, stockage — n'oblige pas à toucher la racine du
corps » — sans qu'aucun mécanisme ne la permette : la struct est `#[non_exhaustive]`, donc
inextensible depuis l'extérieur de la crate.

## La décision structurante

Le code de la sonde **vit dans le projet généré**, jamais dans le noyau.

`rbs-core` ignore `deadpool-redis` et `aws-sdk-s3`, et doit continuer de les ignorer : ses
features cargo `redis`, `mail` et `storage` sont vides, et le commentaire de son
`Cargo.toml` dit pourquoi — « les nommer réserve le nom sans figer, un an à l'avance, les
crates que leur implémentation utilisera ». Le client Redis vit déjà dans le projet, sous
`src/cache/`, engendré par le fragment ; sa sonde y vit aussi.

Le noyau apporte donc la mécanique — le type, la borne de temps, la concurrence, le
verdict — et le projet apporte ce qui varie d'un projet à l'autre : la liste de ses
dépendances et la façon de les joindre.

## L'API du noyau

`health.rs` gagne un type de sonde et une fonction qui les exécute :

```rust
/// Une dépendance à contrôler, sous le nom qu'elle portera dans le corps de la réponse.
pub struct Probe<'a> {
    name: &'static str,
    check: Pin<Box<dyn Future<Output = bool> + Send + 'a>>,
}

impl<'a> Probe<'a> {
    /// Nomme une dépendance et la façon de la joindre.
    ///
    /// Le futur rend `true` quand la dépendance répond. Ce qu'elle a répondu ne regarde
    /// pas le contrôle de santé : une erreur applicative n'est pas une panne.
    pub fn new<F>(name: &'static str, check: F) -> Self
    where
        F: Future<Output = bool> + Send + 'a;
}

/// Rend la santé de l'application, `503` dès qu'une dépendance manque à l'appel.
pub async fn report<C>(db: &C, probes: Vec<Probe<'_>>) -> Response
where
    C: ConnectionTrait;
```

Trois propriétés tenues par le noyau :

- **Chaque sonde est bornée séparément** par `PING_TIMEOUT` (2 s), pour la raison déjà
  écrite en tête du module : un contrôle qui pend laisse l'orchestrateur décider à la
  place du service.
- **Les sondes s'exécutent concurremment**, la base comprise. Quatre dépendances muettes
  répondent en deux secondes, pas en huit — sans quoi ajouter une sonde dégraderait le
  temps de réponse de `/health` pour tout le monde.
- **Une sonde qui échoue vaut 503**, comme la base. Il n'y a pas d'état « dégradé » : le
  verdict reste binaire, et `Status` garde ses deux variantes.

`handler` est conservé tel quel — il délègue à `report` avec une liste vide. Un projet
antérieur continue de compiler sans rien changer.

## La forme du corps

`Checks` gagne un second champ :

```rust
pub struct Checks {
    /// État de la base de données.
    pub database: Check,
    /// État de chaque dépendance sondée par le projet, sous le nom qu'il lui a donné.
    #[serde(flatten)]
    pub extras: BTreeMap<String, Check>,
}
```

Le corps reste plat, `database` garde sa place, et une dépendance ajoutée n'oblige
personne à changer sa sonde de supervision :

```json
{ "status": "unavailable", "checks": { "database": "ok", "cache": "unreachable" } }
```

`BTreeMap` et non `HashMap` : l'ordre du corps est alors stable d'une requête à l'autre,
ce qu'un test d'intégration peut asserter et ce qu'un `diff` entre deux réponses rend
lisible.

**Conséquence sur l'API publique** : `Checks` et `Health` perdent `Copy` et `Eq`. Aucune
fonction publique de `rbs-core` ne rend ni ne prend un `Health` aujourd'hui — `verdict`
est privée, `handler` rend une `Response` — donc aucun code extérieur ne peut détenir la
valeur dont la copie disparaît. La rupture est formelle, pas réelle.

**À vérifier à l'implémentation** : ce que `utoipa` produit pour un `#[serde(flatten)]`
sur une `BTreeMap`. Si le schéma engendré est faux ou refusé, le repli est un
`#[schema(additional_properties)]` explicite, décidé sur la sortie réelle de
`ApiDoc::openapi()` et non sur une supposition.

## L'ancre

Une douzième ancre, `// <rbs:health_probes>`, dans `src/health/controller.rs` — un fichier
déjà engendré par `templates/project/src/health/controller.rs.jinja`, aujourd'hui réduit à
une délégation.

```rust
pub async fn health(State(state): State<AppState>) -> Response {
    rbs_core::health::report(
        state.core().db(),
        vec![
            // <rbs:health_probes>
            // </rbs:health_probes>
        ],
    )
    .await
}
```

Elle rejoint `ANCRES` dans `crates/rbs-cli/src/anchors.rs`, et `doctor` la réclame comme
les onze autres. **Elle manquera sur tout projet engendré avant ce jalon** : c'est
exactement la situation de `layers` à la tâche 53, et le remède est le même — `doctor` la
nomme et affiche son bloc. La tâche #52, menée en parallèle, la reposera par
`doctor --fix`.

`sorted = false` : l'ordre des sondes n'est pas alphabétique mais celui de l'installation,
et rien dans le fichier ne le contraint.

## Ce que chaque fragment inscrit

| Fragment | Sonde | Comment |
|---|---|---|
| `redis` | `cache` | Une méthode `ping()` ajoutée à `Cache` dans `src/cache/mod.rs`, qui prend une connexion du pool et envoie `PING`. |
| `storage` | `storage` | Une méthode sur le trait `Storage` (`src/storage/mod.rs`), implémentée par `s3.rs` — un `head_bucket` — et par `files.rs` — l'accessibilité du répertoire racine. |

Deux fragments, et non quatre :

- **`jobs` n'a pas de sonde.** Sa file est une table de la base ; la sonder reviendrait à
  sonder `database` une seconde fois, sous un autre nom.
- **`mail` n'a pas de sonde.** Ouvrir une connexion SMTP à chaque `/health` — plusieurs
  fois par minute sous un orchestrateur — coûte cher et fait passer le service pour un
  client abusif auprès du relais. L'envoi de courrier est par nature asynchrone : un
  transport momentanément muet n'empêche pas l'API de répondre, et ne justifie pas de
  sortir le pod de la rotation.

Ajouter une sonde plus tard ne demande qu'une ligne dans un `feature.toml` : la mécanique
ne présume pas de cette liste.

## Tests

- **Noyau, unitaires** : `report` avec zéro sonde rend exactement le corps d'aujourd'hui
  (non-régression) ; une sonde fausse fait tomber le verdict d'ensemble à 503 alors que la
  base répond ; deux sondes muettes rendent leur verdict en une fois la borne et non deux,
  sous horloge en pause — c'est la preuve de la concurrence ; l'ordre des clés du corps
  est celui du `BTreeMap`.
- **CLI, rendu** : le `controller.rs` engendré porte l'ancre ; `ANCRES` en compte douze et
  `doctor` les réclame toutes ; les fragments `redis` et `storage` inscrivent leur ligne
  au bon endroit.
- **Intégration** : `rbs new --with redis` puis compilation — le projet engendré doit
  compiler avec sa sonde inscrite, ce qu'aucun test de rendu ne prouve.
- **Exemples** : toute template touchée fait échouer `integration_examples.rs`. Les quatre
  projets d'`examples/` sont à régénérer selon `examples/README.md`, par diff entre deux
  générations et jamais par écrasement.

## Hors périmètre

Un état « dégradé » distinct de `unavailable`, la distinction entre sonde de vivacité et
sonde de disponibilité (`/live` et `/ready`), et la configuration par sonde de sa borne de
temps. Chacune est une tâche à part entière, et aucune n'est nécessaire pour que `/health`
cesse de mentir.
