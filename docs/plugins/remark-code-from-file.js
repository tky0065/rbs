const fs = require('node:fs');
const path = require('node:path');

const DEBUT = /^\s*(?:\/\/|#)\s*region:\s*(\S+)\s*$/;
const FIN = /^\s*(?:\/\/|#)\s*endregion:\s*(\S+)\s*$/;

/// Lit `file=` et `region=` dans la méta d'un bloc de code. Rend `null` pour tout bloc
/// qui ne porte pas `file=` : la documentation en contient d'autres, écrits à la main,
/// qui doivent traverser le plugin sans y toucher.
function analyserMeta(meta) {
  if (!meta) {
    return null;
  }

  const attribut = (nom) => {
    const trouve = meta.match(new RegExp(`${nom}=(?:"([^"]*)"|(\\S+))`));
    return trouve ? (trouve[1] ?? trouve[2]) : null;
  };

  const fichier = attribut('file');
  return fichier ? {fichier, region: attribut('region')} : null;
}

/// Désindente d'après la ligne la moins indentée : une région extraite du corps d'un
/// `impl` arrive sinon décalée de quatre espaces dans la page.
function desindenter(lignes) {
  const marges = lignes
    .filter((ligne) => ligne.trim().length > 0)
    .map((ligne) => ligne.length - ligne.trimStart().length);

  const marge = marges.length > 0 ? Math.min(...marges) : 0;
  return lignes.map((ligne) => ligne.slice(marge)).join('\n');
}

function extraireRegion(source, region, fichier) {
  const lignes = source.split('\n');
  let debut = null;
  let fin = null;

  lignes.forEach((ligne, index) => {
    const ouvre = ligne.match(DEBUT);
    if (ouvre && ouvre[1] === region) {
      if (debut !== null) {
        throw new Error(
          `La région « ${region} » est ouverte deux fois dans ${fichier}. Une région nomme un fragment unique.`,
        );
      }
      debut = index;
      return;
    }

    const ferme = ligne.match(FIN);
    if (ferme && ferme[1] === region && fin === null && debut !== null) {
      fin = index;
    }
  });

  if (debut === null) {
    throw new Error(
      `La région « ${region} » est introuvable dans ${fichier}. Attendu un commentaire « region: ${region} ».`,
    );
  }

  if (fin === null) {
    throw new Error(
      `La région « ${region} » n'est pas refermée dans ${fichier}. Attendu un commentaire « endregion: ${region} ».`,
    );
  }

  return desindenter(lignes.slice(debut + 1, fin));
}

/// Retire les marqueurs quand le fichier entier est cité : ils servent la documentation,
/// le lecteur n'a pas à les voir.
function retirerMarqueurs(source) {
  return source
    .split('\n')
    .filter((ligne) => !DEBUT.test(ligne) && !FIN.test(ligne))
    .join('\n');
}

function parcourir(noeud, visiteur) {
  if (!noeud || typeof noeud !== 'object') {
    return;
  }

  visiteur(noeud);

  for (const enfant of noeud.children ?? []) {
    parcourir(enfant, visiteur);
  }
}

/// Remplit les blocs de code portant `file=` avec le contenu lu sur disque. La spec
/// interdit qu'un extrait de code soit écrit à la main dans le Markdown : c'est ce
/// plugin qui rend la règle tenable, et son échec est dur pour que le site ne se
/// construise jamais sur un extrait périmé.
function codeFromFile({racine} = {}) {
  const base = path.resolve(racine ?? path.join(__dirname, '..', '..'));

  return (arbre, fichierMarkdown) => {
    const page = fichierMarkdown?.path ?? fichierMarkdown?.history?.[0] ?? '(page inconnue)';

    parcourir(arbre, (noeud) => {
      if (noeud.type !== 'code') {
        return;
      }

      const demande = analyserMeta(noeud.meta);
      if (!demande) {
        return;
      }

      const absolu = path.resolve(base, demande.fichier);
      if (absolu !== base && !absolu.startsWith(base + path.sep)) {
        throw new Error(
          `${page} cite « ${demande.fichier} », hors de la racine du dépôt (${base}).`,
        );
      }

      let source;
      try {
        source = fs.readFileSync(absolu, 'utf8');
      } catch {
        throw new Error(
          `${page} cite « ${demande.fichier} », introuvable. Le fichier a-t-il été déplacé ou l'exemple régénéré ?`,
        );
      }

      noeud.value = demande.region
        ? extraireRegion(source, demande.region, demande.fichier)
        : retirerMarqueurs(source).trimEnd();
    });
  };
}

module.exports = codeFromFile;
module.exports.codeFromFile = codeFromFile;
module.exports.analyserMeta = analyserMeta;
module.exports.extraireRegion = extraireRegion;
