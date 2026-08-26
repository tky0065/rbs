const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const {afterEach, beforeEach, describe, it} = require('node:test');

const {analyserMeta, extraireRegion, codeFromFile} = require('../remark-code-from-file');

describe('analyserMeta', () => {
  it('ne reconnaît que les métas portant file=', () => {
    assert.equal(analyserMeta(null), null);
    assert.equal(analyserMeta(''), null);
    assert.equal(analyserMeta('title="autre chose"'), null);
  });

  it('lit le fichier et la région', () => {
    assert.deepEqual(analyserMeta('file=examples/a/src/main.rs region=router'), {
      fichier: 'examples/a/src/main.rs',
      region: 'router',
    });
  });

  it('accepte un fichier sans région', () => {
    assert.deepEqual(analyserMeta('file=examples/a/src/main.rs'), {
      fichier: 'examples/a/src/main.rs',
      region: null,
    });
  });

  it('accepte les valeurs entre guillemets', () => {
    assert.deepEqual(analyserMeta('file="examples/mon projet/main.rs"'), {
      fichier: 'examples/mon projet/main.rs',
      region: null,
    });
  });
});

describe('extraireRegion', () => {
  const source = [
    'use axum::Router;',
    '',
    '// region: router',
    'pub fn router() -> Router {',
    '    Router::new()',
    '}',
    '// endregion: router',
    '',
    'fn autre() {}',
  ].join('\n');

  it('rend les lignes entre les marqueurs, marqueurs exclus', () => {
    assert.equal(
      extraireRegion(source, 'router', 'x.rs'),
      'pub fn router() -> Router {\n    Router::new()\n}',
    );
  });

  it('désindente la région selon sa ligne la moins indentée', () => {
    const imbriquee = [
      'impl A {',
      '    // region: methode',
      '    fn f(&self) {',
      '        ();',
      '    }',
      '    // endregion: methode',
      '}',
    ].join('\n');
    assert.equal(extraireRegion(imbriquee, 'methode', 'x.rs'), 'fn f(&self) {\n    ();\n}');
  });

  it('échoue si la région est absente', () => {
    assert.throws(() => extraireRegion(source, 'fantome', 'x.rs'), /fantome/);
  });

  it('échoue si la région n\'est pas refermée', () => {
    const ouverte = '// region: a\nlet x = 1;';
    assert.throws(() => extraireRegion(ouverte, 'a', 'x.rs'), /refermée|endregion/);
  });

  it('échoue si la région est ouverte deux fois', () => {
    const doublon = '// region: a\n1\n// endregion: a\n// region: a\n2\n// endregion: a';
    assert.throws(() => extraireRegion(doublon, 'a', 'x.rs'), /deux fois|plusieurs/);
  });
});

describe('codeFromFile', () => {
  let racine;

  beforeEach(() => {
    racine = fs.mkdtempSync(path.join(os.tmpdir(), 'rbs-snippets-'));
    fs.mkdirSync(path.join(racine, 'examples'), {recursive: true});
    fs.writeFileSync(
      path.join(racine, 'examples', 'main.rs'),
      'fn main() {}\n// region: corps\nlet x = 1;\n// endregion: corps\n',
    );
  });

  afterEach(() => {
    fs.rmSync(racine, {recursive: true, force: true});
  });

  const transformer = (arbre) => {
    codeFromFile({racine})(arbre, {path: 'docs/page.md'});
    return arbre;
  };

  it('remplit un bloc vide avec le fichier entier', () => {
    const noeud = {type: 'code', lang: 'rust', meta: 'file=examples/main.rs', value: ''};
    transformer({type: 'root', children: [noeud]});

    assert.match(noeud.value, /fn main\(\) \{\}/);
    assert.doesNotMatch(noeud.value, /region: corps/);
  });

  it('remplit un bloc avec la seule région demandée', () => {
    const noeud = {
      type: 'code',
      lang: 'rust',
      meta: 'file=examples/main.rs region=corps',
      value: '',
    };
    transformer({type: 'root', children: [noeud]});

    assert.equal(noeud.value, 'let x = 1;');
  });

  it('descend dans les nœuds imbriqués', () => {
    const noeud = {type: 'code', lang: 'rust', meta: 'file=examples/main.rs', value: ''};
    transformer({
      type: 'root',
      children: [{type: 'blockquote', children: [{type: 'list', children: [noeud]}]}],
    });

    assert.match(noeud.value, /fn main/);
  });

  it('laisse intact un bloc de code ordinaire', () => {
    const noeud = {type: 'code', lang: 'rust', meta: null, value: 'écrit à la main'};
    transformer({type: 'root', children: [noeud]});

    assert.equal(noeud.value, 'écrit à la main');
  });

  it('échoue si le fichier est absent, en nommant la page fautive', () => {
    const noeud = {type: 'code', lang: 'rust', meta: 'file=examples/absent.rs', value: ''};
    assert.throws(
      () => transformer({type: 'root', children: [noeud]}),
      /absent\.rs[\s\S]*docs\/page\.md|docs\/page\.md[\s\S]*absent\.rs/,
    );
  });

  it('échoue si la région est absente', () => {
    const noeud = {
      type: 'code',
      lang: 'rust',
      meta: 'file=examples/main.rs region=fantome',
      value: '',
    };
    assert.throws(() => transformer({type: 'root', children: [noeud]}), /fantome/);
  });

  it('refuse de sortir de la racine du dépôt', () => {
    const noeud = {
      type: 'code',
      lang: 'rust',
      meta: 'file=../../../etc/passwd',
      value: '',
    };
    assert.throws(() => transformer({type: 'root', children: [noeud]}), /racine|hors/);
  });
});
