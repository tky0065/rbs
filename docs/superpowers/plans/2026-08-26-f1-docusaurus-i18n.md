# Docusaurus + i18n — plan d'implémentation

**Goal:** Un site de documentation qui se construit et bascule entre l'anglais et le
français, socle de toutes les pages de contenu du lot F.

**Architecture:** La spec §D6 a déjà tranché la technologie — Docusaurus, i18n intégré,
FR + EN — contre mdBook en deux livres et le Markdown brut. `F1` n'est que l'exécution de
cette décision. Deux contraintes locales la cadrent :

`docs/` n'est pas vide : `docs/superpowers/{specs,plans}` y vivent. Ce sont des documents
de travail, jamais des pages du site — Docusaurus doit les ignorer explicitement, et non
par l'effet de bord de son périmètre de scan par défaut.

`F9` a été cochée sur une preuve exécutée, `grep -c 'npm\|node\|yarn' .github/workflows/ci.yml`
→ 0, adossée à la garantie « contribuer au code sans installer Node » que la spec §7 pose
comme mitigation du risque « toolchain Node dans un dépôt Rust ». Toute étape Node ajoutée
à `ci.yml` invaliderait cette preuve rétroactivement. Le site prend donc son propre
workflow.

Langue par défaut `en`, locale additionnelle `fr` — alignement sur la convention déjà
établie par `CONTRIBUTING.md` / `CONTRIBUTING.fr.md`, décision du user 2026-08-26. URLs
`/` et `/fr/`. Ce choix se change mal une fois des liens externes en place, d'où sa
validation avant écriture.

**Spec:** `docs/superpowers/specs/2026-08-25-rbs-design.md` §D6, §7.

## Global Constraints

- Branche dédiée `f1-docusaurus-i18n`.
- Docusaurus 3.10.2, preset `classic`, TypeScript. Ni blog ni versioning des docs : la
  v0.1 n'a rien à versionner.
- `F1` ne rédige pas les pages de `F3`–`F7`. Le contenu se limite au strict nécessaire
  pour prouver que la bascule fonctionne.
- Les deux langues dans le même commit, règle du CLAUDE.md.
- `docs/node_modules` et `docs/build` hors du dépôt.

## File Structure

- Create: `docs/package.json`, `docs/docusaurus.config.ts`, `docs/sidebars.ts`,
  `docs/tsconfig.json`, `docs/src/css/custom.css`, `docs/docs/`, `docs/i18n/fr/`,
  `.github/workflows/docs.yml`
- Modify: `.gitignore`

---

### Task 1: Socle Docusaurus bilingue

- [ ] **Step 1:** Échafauder Docusaurus 3.10.2 dans `docs/`, en préservant
      `docs/superpowers/`.
- [ ] **Step 2:** Configurer `i18n` — `defaultLocale: 'en'`, `locales: ['en', 'fr']`,
      libellés de chaque locale — et poser le `localeDropdown` dans la barre de
      navigation.
- [ ] **Step 3:** Exclure `superpowers/` du périmètre scanné, explicitement.
- [ ] **Step 4:** Réduire le contenu échafaudé : retirer blog, pages et tutoriels de
      démonstration ; poser une page d'accueil et une page d'index dans les deux langues.
- [ ] **Step 5:** Traduire les chaînes d'interface (`npm run write-translations -- --locale fr`)
      et renseigner le fichier `code.json` français.
- [ ] **Step 6:** `npm run build` — lire la sortie réelle, vérifier que `build/index.html`
      et `build/fr/index.html` existent et diffèrent.
- [ ] **Step 7:** Workflow `docs.yml` déclenché sur `docs/**`, `npm ci && npm run build`.
      Vérifier que `ci.yml` reste à 0 occurrence de Node.
- [ ] **Step 8:** Faire valider le sélecteur de langue par le user sur `npm run serve` —
      critère visuel, jamais auto-décerné.
- [ ] **Step 9:** Cocher F1 dans `TODO.md`, avec sa preuve sur une ligne.
- [ ] **Step 10: Commit** — `docs: initialise le site Docusaurus bilingue`
