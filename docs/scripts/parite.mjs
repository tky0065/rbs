// Mesure la parité FR/EN de la documentation. La règle du projet — les deux langues dans
// le même commit — se vérifie ici plutôt qu'à la lecture : une revue comparée de quinze
// pages rate ce qu'un profil structurel voit d'un coup.
//
//   node scripts/parite.mjs
//
// Le texte des titres est traduit, donc jamais comparé. Ce qui doit coïncider d'une
// langue à l'autre est ce que la traduction ne touche pas : la charpente des titres, la
// langue et la méta des blocs de code, le type des encarts, la cible des liens relatifs.
//
// Deux jeux de fichiers passent sous la même comparaison :
//
//   · les pages du site, « docs/<chemin>.md » contre « i18n/fr/…/current/<chemin>.md » ;
//   · les fichiers racine du dépôt, « X.md » contre « X.fr.md ».
//
// Le dernier commit n'est comparé que sur les pages du site. À la racine, les deux
// moitiés d'une paire n'ont pas toujours été écrites ensemble — « CODE_OF_CONDUCT.fr.md »
// est né bien après son homologue — et cette histoire ne se réécrit pas.

import {execFileSync} from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import {fileURLToPath} from 'node:url';

const DOCS = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const EN = path.join(DOCS, 'docs');
const FR = path.join(DOCS, 'i18n/fr/docusaurus-plugin-content-docs/current');
const RACINE = path.join(DOCS, '..');

// Documents de travail du dépôt, écrits en français pour qui y contribue : ils n'ont pas
// d'homologue anglais et n'en attendent pas. Tout autre « .md » de la racine s'adresse à
// qui installe rbs, et la liste étant close, un fichier neuf y est réclamé dans les deux
// langues plutôt que laissé à l'appréciation d'une revue.
const RACINE_MONOLINGUE = new Set(['CLAUDE.md', 'ROADMAP.md', 'TODO.md']);

function pages(racine, prefixe = '') {
  return fs
    .readdirSync(path.join(racine, prefixe), {withFileTypes: true})
    .flatMap((entree) => {
      const relatif = path.join(prefixe, entree.name);
      if (entree.isDirectory()) return pages(racine, relatif);
      return entree.name.endsWith('.md') ? [relatif] : [];
    })
    .sort();
}

/// La charpente d'une page : ce qui doit survivre à la traduction.
function profil(source) {
  const titres = [];
  const blocs = [];
  const encarts = [];
  const liens = [];
  let cloture = null;

  for (const ligne of source.split('\n')) {
    const barriere = ligne.match(/^(\s*)(`{3,}|~{3,})(.*)$/);
    if (barriere) {
      const [, , delimiteur, reste] = barriere;
      if (cloture === null) {
        cloture = delimiteur[0].repeat(delimiteur.length);
        // « ```rust file=… region=… » : la langue et la méta nomment la source, et la
        // traduction n'a rien à y changer.
        blocs.push(reste.trim());
      } else if (delimiteur.startsWith(cloture) && reste.trim() === '') {
        cloture = null;
      }
      continue;
    }
    if (cloture !== null) continue;

    const titre = ligne.match(/^(#{1,6})\s/);
    if (titre) titres.push(titre[1].length);

    const encart = ligne.match(/^:::(\w+)/);
    if (encart) encarts.push(encart[1]);

    for (const lien of ligne.matchAll(/\]\(([^)\s]+\.md)(#[^)\s]*)?\)/g)) {
      // Une URL absolue vers un dépôt se termine aussi par « .md » sans être un lien
      // interne : elle n'a pas de fichier à résoudre de ce côté-ci.
      if (!/^(https?:)?\/\//.test(lien[1])) liens.push(lien[1]);
    }
  }

  return {titres, blocs, encarts, liens, cloture};
}

function dernierCommit(fichier) {
  return execFileSync('git', ['log', '-1', '--format=%H', '--', fichier], {
    cwd: DOCS,
    encoding: 'utf8',
  }).trim();
}

function comparer(nom, attendu, obtenu) {
  if (attendu.length !== obtenu.length) {
    return [`${nom} : ${attendu.length} en anglais, ${obtenu.length} en français`];
  }
  return attendu.flatMap((valeur, index) =>
    valeur === obtenu[index]
      ? []
      : [`${nom} nº${index + 1} : « ${valeur} » en anglais, « ${obtenu[index]} » en français`],
  );
}

/// « CONTRIBUTING.fr.md » et « CONTRIBUTING.md » nomment le même document : c'est sous ce
/// nom commun que les deux moitiés d'une paire racine comparent leurs liens, sans quoi
/// tout renvoi d'un fichier à l'autre passerait pour un écart.
function sansLangue(nom) {
  return nom.replace(/\.fr\.md$/, '.md');
}

function fichiersRacine() {
  return fs
    .readdirSync(RACINE, {withFileTypes: true})
    .filter((entree) => entree.isFile() && entree.name.endsWith('.md'))
    .map((entree) => entree.name)
    .sort();
}

const enPages = pages(EN);
const frPages = pages(FR);
const ecarts = [];

for (const relatif of enPages) {
  if (!frPages.includes(relatif)) ecarts.push(`${relatif} : aucune version française`);
}
for (const relatif of frPages) {
  if (!enPages.includes(relatif)) ecarts.push(`${relatif} : aucune version anglaise`);
}

const paires = enPages.filter((relatif) => frPages.includes(relatif));
let memeCommit = 0;

for (const relatif of paires) {
  const en = profil(fs.readFileSync(path.join(EN, relatif), 'utf8'));
  const fr = profil(fs.readFileSync(path.join(FR, relatif), 'utf8'));

  for (const [langue, page] of [['anglaise', en], ['française', fr]]) {
    if (page.cloture !== null) {
      ecarts.push(`${relatif} : un bloc de code n'est pas refermé dans la version ${langue}`);
    }
  }

  const differences = [
    ...comparer('niveau de titre', en.titres, fr.titres),
    ...comparer('bloc de code', en.blocs, fr.blocs),
    ...comparer('encart', en.encarts, fr.encarts),
    ...comparer('lien relatif', en.liens, fr.liens),
  ];
  ecarts.push(...differences.map((difference) => `${relatif} : ${difference}`));

  const commitEn = dernierCommit(path.join('docs', relatif));
  const commitFr = dernierCommit(path.join('i18n/fr/docusaurus-plugin-content-docs/current', relatif));
  if (commitEn === commitFr) {
    memeCommit++;
  } else {
    ecarts.push(
      `${relatif} : dernier commit ${commitEn.slice(0, 7)} en anglais, ` +
        `${commitFr.slice(0, 7)} en français — les deux langues voyagent ensemble`,
    );
  }
}

const liensMorts = paires.flatMap((relatif) =>
  [['docs', EN], ['i18n/fr/…/current', FR]].flatMap(([etiquette, racine]) => {
    const {liens} = profil(fs.readFileSync(path.join(racine, relatif), 'utf8'));
    return liens
      .filter((lien) => !fs.existsSync(path.resolve(path.dirname(path.join(racine, relatif)), lien)))
      .map((lien) => `${etiquette}/${relatif} : « ${lien} » ne résout vers aucun fichier`);
  }),
);
ecarts.push(...liensMorts);

const racineTous = fichiersRacine();
const pairesRacine = [];

for (const nom of racineTous) {
  if (nom.endsWith('.fr.md')) {
    if (!racineTous.includes(sansLangue(nom))) ecarts.push(`${nom} : aucune version anglaise`);
    continue;
  }
  if (RACINE_MONOLINGUE.has(nom)) continue;

  const fr = nom.replace(/\.md$/, '.fr.md');
  if (racineTous.includes(fr)) pairesRacine.push([nom, fr]);
  else ecarts.push(`${nom} : aucune version française`);
}

let liensRacine = 0;

for (const [nomEn, nomFr] of pairesRacine) {
  const en = profil(fs.readFileSync(path.join(RACINE, nomEn), 'utf8'));
  const fr = profil(fs.readFileSync(path.join(RACINE, nomFr), 'utf8'));
  liensRacine += en.liens.length + fr.liens.length;

  for (const [nom, page] of [[nomEn, en], [nomFr, fr]]) {
    if (page.cloture !== null) ecarts.push(`${nom} : un bloc de code n'est pas refermé`);
  }

  const differences = [
    ...comparer('niveau de titre', en.titres, fr.titres),
    ...comparer('bloc de code', en.blocs, fr.blocs),
    ...comparer('encart', en.encarts, fr.encarts),
    ...comparer('lien relatif', en.liens.map(sansLangue), fr.liens.map(sansLangue)),
  ];
  ecarts.push(...differences.map((difference) => `${nomEn} ↔ ${nomFr} : ${difference}`));

  // Le nom commun rapproche « CODE_OF_CONDUCT.md » et « CODE_OF_CONDUCT.fr.md », donc la
  // comparaison ci-dessus ne voit pas un fichier français qui renverrait vers l'anglais.
  // Seule exception, l'autre moitié de sa propre paire : c'est le sélecteur de langue.
  for (const lien of fr.liens) {
    if (lien === nomEn || lien.endsWith('.fr.md')) continue;
    if (racineTous.includes(lien.replace(/\.md$/, '.fr.md'))) {
      ecarts.push(`${nomFr} : « ${lien} » renvoie vers la version anglaise`);
    }
  }

  for (const [nom, page] of [[nomEn, en], [nomFr, fr]]) {
    for (const lien of page.liens) {
      if (!fs.existsSync(path.resolve(RACINE, lien))) {
        ecarts.push(`${nom} : « ${lien} » ne résout vers aucun fichier`);
      }
    }
  }
}

console.log(`${paires.length} paires de pages, ${memeCommit}/${paires.length} au même dernier commit`);
const totalLiens = paires.reduce(
  (total, relatif) => total + profil(fs.readFileSync(path.join(EN, relatif), 'utf8')).liens.length,
  0,
);
console.log(`${totalLiens} liens relatifs en anglais, autant attendus en français`);
console.log(`${pairesRacine.length} paires de fichiers racine, ${liensRacine} liens relatifs résolus`);

if (ecarts.length === 0) {
  console.log('0 écart structurel');
  process.exit(0);
}

console.log(`\n${ecarts.length} écart(s) :`);
for (const ecart of ecarts) console.log(`  ${ecart}`);
process.exit(1);
