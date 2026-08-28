# `integration_storage` sous MinIO

## Ce qui s'ajoute

- `templates/features/storage/tests.rs.jinja` : deux tests `#[ignore]` — `ronde` rejouée
  contre S3, et un dépôt relu par un client construit à part.
- `crates/rbs-cli/tests/integration_storage.rs` : MinIO en `GenericImage`, la surcharge
  des `RBS_STORAGE__*`, et l'assertion que les deux tests ont tourné.

## Ce que N1 avait déjà préparé

`ronde(&dyn Storage)` existe depuis `N1`, écrite contre le trait et annotée « rejouable
telle quelle contre S3 : deux backends qui ne passeraient pas la même ronde n'abstrairaient
rien ». Le premier critère ne demande donc **aucune ligne de test nouvelle** : il demande
de rejouer celle-là. C'est précisément ce qui lui donne sa valeur — un jeu réécrit pour S3
prouverait que S3 marche, pas que le trait abstrait.

## Décisions

- **La relecture hors du trait construit son propre client.** Le champ `client` de
  `StockageS3` est privé au module `s3` : le test ne peut pas le réemprunter, et le chemin
  de relecture est donc réellement indépendant plutôt que nominalement. « Hors du trait »
  s'oppose à « via le trait », non à « depuis le dépôt ».
- **Le bucket est fourni par le conteneur, pas créé par un test livré.** MinIO démarre sur
  `sh -c "mkdir -p /data/<bucket> && minio server /data"` — un répertoire de premier niveau
  de `/data` *est* un bucket. Le fragment se contente alors de déposer et de lire, comme
  face à un bucket de production, et aucun test livré à l'utilisateur ne crée de ressource.
- **Les identifiants passent par l'environnement**, comme la section l'impose déjà :
  `RBS_STORAGE__ACCESS_KEY_ID` et `RBS_STORAGE__SECRET_ACCESS_KEY` sont dans `.env.example`
  depuis `N2`, jamais dans `config/`.
- `force_path_style` à `true` : MinIO veut le bucket dans le chemin, ce que le champ
  documente déjà.

## Le faux vert à fermer

Comme en `L3` et `M3` : `cargo test -- --ignored` sort en 0 sans avoir filtré aucun test.
Les deux noms sont cherchés dans le journal.

## Ordre

1. Les deux tests du fragment, `integration_storage.rs`, lancés → échec.
2. Vert, puis clippy et rustfmt sur le dépôt et sur un projet réel.
3. Morsures : `lire` de `StockageS3` rendant un contenu constant (la ronde et la relecture
   doivent tomber) ; `deposer` sans effet (les deux aussi, mais sur l'existence) ;
   `normaliser` retiré du chemin S3 — à observer, la ronde n'employant pas de clé piégée.
