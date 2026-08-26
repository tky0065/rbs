# A6 — Capture du formateur `pretty` dans les docs

Reste de la tâche : le second volet du critère « inspection visuelle sur les cinq
niveaux **+ capture dans les docs** ». Les tests et l'inspection sont acquis
(2026-08-25) ; la capture attendait le site, livré par F1.

## Décision

PNG d'un vrai terminal, **doublé** d'un bloc extrait de `logs_pretty.rs` par le
plugin de F2. Le PNG montre ce qu'un bloc de code ne peut pas — la couleur et
l'alignement des colonnes ; l'extrait, lui, ne peut pas dériver du code réel.

La dérive du PNG est atténuée en le rendant **régénérable** : un script versionné
le produit depuis la sortie réelle de l'exemple, personne ne le retouche à la main.

## Étapes

1. `docs/scripts/capture_logs_pretty.py` — lance l'exemple sous `script(1)` (pty,
   sinon le formateur retire les couleurs, cf. le test d'A6), analyse les séquences
   SGR, rend en PNG via Pillow. Écrit `docs/static/img/logs-pretty.png`.
2. Page `docs/docs/guides/logs.md` (EN) + `docs/i18n/fr/.../guides/logs.md` (FR) :
   le PNG, l'extrait `region=niveaux`, le code couleur des cinq niveaux.
   Volontairement minimale — F6 l'étoffera (bascule `json`, `RUST_LOG`).
3. Région `niveaux` posée dans `crates/rbs-core/examples/logs_pretty.rs`.

## Preuves attendues

- Les cinq niveaux présents et distincts dans le PNG → **validation du user**
  (critère visuel : ne pas se l'auto-décerner).
- `npm run build` → `[SUCCESS]` sur `en` et `fr`, image et région résolues.
- Le garde-fou de F2 mord toujours : région retirée → build en échec.
