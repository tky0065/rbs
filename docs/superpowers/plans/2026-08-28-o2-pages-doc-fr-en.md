# O2 — Pages de documentation FR et EN

## Décision

Trois guides plutôt qu'une page des « trois briques » : le cache, le courriel et le
stockage n'ont rien en commun que d'être arrivés dans le même jalon. Chacun suit le plan
d'`auth.md`, que `J3` a validé.

La contrainte n'est pas rédactionnelle, elle est dans `integration_examples` :
`edite_a_la_main` est la liste des fichiers **exclus** de la comparaison de dérive, et
`the_hand_edits_of_file_drop_are_in_place` compense en assertant leur contenu. Y allonger
un chemin pour la commodité d'une région sortirait ce fichier de toute surveillance. D'où
trois régimes de citation, dont aucun n'allonge cette liste ni ne pose de marqueur dans
les templates du CLI :

| Régime | Fichiers |
|---|---|
| Régions libres — déjà hors comparaison | `cache/mod.rs`, `mail/mod.rs`, `mail/service.rs`, `storage/mod.rs`, `uploads/{service,controller,mod}.rs`, `templates/mail/depot.html` |
| Cités entiers, `file=` sans `region=` | `cache/config.rs`, `mail/config.rs`, `mail/template.rs`, `storage/files.rs`, `.env.example`, `config/default.toml` |
| Prose et lien, aucun bloc de code | `cache/tests.rs`, `mail/tests.rs`, `storage/tests.rs`, `storage/s3.rs` |

Le troisième régime est celui qu'`auth.md` applique déjà à `src/auth/tests.rs`. Le critère
interdit d'inventer du code, non d'en parler : payer 250 lignes de bloc ou affaiblir la
garantie de non-dérive coûterait plus qu'il ne prouve.

## Étapes

1. Poser les régions manquantes dans les fichiers du premier régime, et vérifier
   aussitôt que `integration_examples` reste à 14.
2. Écrire l'instrument de parité et l'éprouver par morsures **avant** qu'il ne serve de
   preuve : titre retiré du FR, méta de bloc changée, page FR absente.
3. `guides/cache.md`, `guides/mail.md`, `guides/storage.md` en anglais, puis leurs trois
   miroirs français dans le même commit.
4. `cli/add.md` EN et FR : la page annonce trois features quand le CLI en livre six. Hors
   des critères d'`O2`, corrigé en passant et consigné comme tel.

## Preuve

L'instrument de parité sur les paires du site, `npm run clear && npm run build` sur les
deux locales — c'est lui qui tient le second critère, le plugin échouant sur tout `file=`
ou `region=` faux —, `write-translations` sans entrée vide, et
`integration_examples` inchangé.
