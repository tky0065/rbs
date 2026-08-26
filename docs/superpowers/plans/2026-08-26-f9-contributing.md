# CONTRIBUTING et code de conduite — plan d'implémentation

**Goal:** Qu'un contributeur sache où poser sa première ligne, et que CONTRIBUTING indique
explicitement comment contribuer au code **sans installer Node**.

**Architecture:** Le critère vient d'un risque identifié en spec §7 : « Toolchain Node dans
un dépôt Rust → friction pour les contributeurs → `docs/` isolé, CI séparée, contribution
au code possible sans Node ». CONTRIBUTING doit donc énoncer cette séparation comme une
garantie, pas la laisser déduire.

Langue : anglais d'abord, `CONTRIBUTING.fr.md` en regard — décision du user 2026-08-26,
cohérente avec F7 qui prévoit un README FR + EN. Le code de conduite reprend Contributor
Covenant 2.1, texte inchangé.

**Spec:** `docs/superpowers/specs/2026-08-25-rbs-design.md` §5.5, §5.6, §7.

## Global Constraints

- Branche dédiée `f9-contributing`.
- Les commandes citées dans CONTRIBUTING **sont lancées** avant d'être écrites. Un
  CONTRIBUTING qui donne une commande fausse est pire que pas de CONTRIBUTING.
- Les deux langues dans le même commit, règle du CLAUDE.md.
- Conventional Commits, branche dédiée, ordre des lots : ce que le dépôt s'impose,
  CONTRIBUTING le documente.

## File Structure

- Create: `CONTRIBUTING.md`, `CONTRIBUTING.fr.md`, `CODE_OF_CONDUCT.md`

---

### Task 1: CONTRIBUTING

- [ ] **Step 1:** Lancer les commandes destinées à y figurer, et noter leur sortie réelle.
- [ ] **Step 2:** Rédiger EN, puis FR — build, tests, les trois niveaux de test, le fait
      que les tests d'intégration demandent Docker, la convention de commits, et la
      section « Contributing without Node » qui donne la garantie de la spec §7.
- [ ] **Step 3:** Déposer Contributor Covenant 2.1, contact `tky0065@gmail.com`.
- [ ] **Step 4:** Relire la parité EN/FR ligne à ligne.
- [ ] **Step 5:** Cocher F9 dans `TODO.md`, avec sa preuve sur une ligne.
- [ ] **Step 6: Commit** — `docs: ajoute le guide de contribution et le code de conduite`
