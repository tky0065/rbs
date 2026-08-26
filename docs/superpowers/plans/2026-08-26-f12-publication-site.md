# F12 — Publication du site sur GitHub Pages

## Décision

`docs.yml` construit déjà le site sans le publier. La configuration Docusaurus vise
`https://tky0065.github.io/rbs/` : l'essentiel du travail est un job `deploy`.

Le filtre `paths:` du workflow ignorait `examples/`, d'où le site tire pourtant tous ses
extraits (F2). Une modification d'un exemple changeait le contenu rendu sans déclencher de
reconstruction : le site publié aurait servi du code périmé, silencieusement. Un
déploiement qui sert de l'obsolète est un défaut de ce que cette tâche livre, pas un
à-côté — le filtre gagne `examples/**`.

## Étapes

1. `paths:` de `push` et de `pull_request` : ajouter `examples/**`.
2. Job `build` : ajouter `actions/upload-pages-artifact`, `path: docs/build`. Le
   `defaults.run.working-directory: docs` ne vaut que pour les étapes `run` — le `with:`
   d'une action se résout depuis la racine du dépôt.
3. Job `deploy` : `needs: build`, conditionné à un push sur `main`, `environment:
   github-pages`, `actions/deploy-pages`.
4. Permissions au niveau du job `deploy` seul (`pages: write`, `id-token: write`), pas du
   workflow : le build n'en a pas besoin.
5. Concurrence propre au job `deploy` : groupe `pages`, `cancel-in-progress: false`. Le
   groupe existant annule les runs en cours, ce qui convient à un build et détruit un
   déploiement interrompu en vol.

## Preuve

- `npm run build` produit `docs/build` avec les deux locales.
- Le YAML est valide et les deux jobs sont reconnus.

Le déploiement lui-même n'est **pas** prouvable ici : `gh api repos/tky0065/rbs/pages`
répond `404` — la source Pages n'est pas activée, et l'activer est une écriture sur
GitHub, exclue par la contrainte « local seulement » retenue pour ce lot. La case reste
`- [ ]` avec une annotation `PARTIEL`.
