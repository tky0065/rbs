# Modèles d'issues et de PR — plan d'implémentation

**Goal:** Qu'un rapport de bug arrive avec la version de rbs, la commande lancée et la
sortie obtenue, sans que le mainteneur ait à les réclamer.

**Architecture:** Markdown classique avec front-matter — décision du user 2026-08-26. Aucun
champ n'est donc imposé par GitHub : le modèle compense en posant des questions fermées
plutôt qu'en ouvrant des rubriques. Deux modèles, bug et évolution, plus un modèle de PR.

Le champ qui compte pour ce projet est la **sortie de `rbs doctor`** : elle porte d'un coup
les ancres, le `.env`, la joignabilité de la base et la cohérence des versions — soit les
quatre premières questions que le mainteneur poserait.

## Global Constraints

- Branche dédiée `f11-modeles-issues`.
- Anglais, cohérent avec F9.
- Un modèle de PR qui réclame les commandes lancées et leur résultat : c'est déjà la règle
  des commits du dépôt, elle ne doit pas s'arrêter aux contributions extérieures.

## File Structure

- Create: `.github/ISSUE_TEMPLATE/bug_report.md`
- Create: `.github/ISSUE_TEMPLATE/feature_request.md`
- Create: `.github/ISSUE_TEMPLATE/config.yml` — issues vierges désactivées
- Create: `.github/PULL_REQUEST_TEMPLATE.md`

---

### Task 1: Les quatre modèles

- [ ] **Step 1:** Écrire les deux modèles d'issues, le `config.yml` et le modèle de PR.
- [ ] **Step 2:** Vérifier que chaque front-matter est du YAML valide et que les
      `labels` cités existent ou sont créables.
- [ ] **Step 3:** Cocher F11 dans `TODO.md`, avec sa preuve sur une ligne.
- [ ] **Step 4: Commit** — `docs: ajoute les modèles d'issues et de pull request`
