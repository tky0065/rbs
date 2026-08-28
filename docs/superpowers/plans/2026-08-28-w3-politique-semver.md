# W3 — La politique semver, README et docs FR/EN

**Conception.** Le gel est posé et mesuré ; il reste à l'énoncer. Une promesse que le dépôt
tient sans la dire ne sert personne : c'est ce que le lecteur lit qui décide s'il épingle
une version exacte ou s'il fait confiance à `^1`.

**Les cinq périmètres viennent de la conception §2.4 et ne se rediscutent pas :**

| Périmètre | Couvert | Pourquoi |
|---|---|---|
| API publique de `rbs-core` | **oui** | C'est ce que le `ROADMAP` gèle |
| Format des ancres et de `[package.metadata.rbs]` | **oui** | Un projet engendré en 0.4.0 doit rester lisible par `rbs add` en 1.x |
| Le code engendré dans le projet | non | Il appartient à l'utilisateur dès qu'il est écrit ; le CLI ne le relit pas |
| La bibliothèque de `rbs-cli` | non | `rbs-cli` publie un binaire ; que sa lib soit visible est un détail de construction |
| Les features vides `redis`, `mail`, `storage` | non | Elles réservent un nom et ne portent aucun code |

**Le point non évident est le second, et c'est lui qui mérite le plus d'explication.** Les
ancres et `[package.metadata.rbs]` ne sont pas une API Rust ; on les oublierait naturellement
d'une promesse de compatibilité. Or un projet engendré en 0.4.0 qui deviendrait illisible
par `rbs add` en 1.x mourrait à la première mise à jour du CLI — et son propriétaire n'aurait
aucun moyen de le voir venir.

**La page vit à la racine du site**, `docs/docs/compatibility.md` et son miroir français,
près d'`architecture.md` : elle dit ce que le projet engage, non comment faire quelque chose.
Les guides répondent à « comment faire X » ; celle-ci répond à « à quoi puis-je me fier ».

**Le README passe à l'état d'après-publication**, comme au lot `U` : il annonce la promesse
au lieu de son absence. La 1.0.0 n'est pas encore publiée quand cette tâche s'écrit —
l'inversion est la même qu'à `U2` et l'arbitrage a été rendu de la même façon. `Y2` la
publie.

## Étapes

1. Écrire `docs/docs/compatibility.md` : les cinq périmètres, chacun avec sa raison. Donner
   au second le développement qu'il mérite.
2. Écrire le miroir français, `docs/i18n/fr/docusaurus-plugin-content-docs/current/compatibility.md`,
   dans le même commit.
3. Réécrire la section « Status » / « Statut » des deux README : la promesse remplace son
   absence, avec un renvoi à la nouvelle page.
4. Preuves : `npm run parite` sur les paires du site **et** les paires racine, 0 écart ;
   l'instrument éprouvé avant de servir de preuve ; `npm run build` du site.
