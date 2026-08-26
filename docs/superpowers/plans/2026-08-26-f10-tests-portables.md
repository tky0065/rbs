# F10 (suite) — Deux tests non portables révélés par la matrice

## Constat

Le premier run de la matrice a donné Linux vert, macOS vert, Windows rouge, sur deux
tests. Le code de production n'est pas en cause : `dependance_noyau` passe le chemin par
`toml_edit::Value::from`, qui échappe correctement les antislashs d'un chemin Windows. Ce
sont les deux assertions qui présument que les chemins s'écrivent avec `/`.

- `new.rs` refabriquait le TOML attendu à la main, sans l'échappement que la production
  applique : sous Windows le manifeste porte des antislashs doublés, l'attendu des simples.
- `templates.rs` comparait à la chaîne `"config/default.toml"`, là où `to_string_lossy`
  rend `config\default.toml`.

Dans les deux cas la même faute : comparer la *représentation textuelle* d'un chemin là où
c'est son *sens* qui compte.

## Correctifs

1. `new.rs` — analyser le manifeste et comparer `dependencies.rbs-core.path` au chemin
   absolu attendu. Refabriquer l'attendu avec `toml_edit` aurait été tautologique : le
   test rejouerait le code testé. L'analyse est aussi plus forte que `contains`, puisqu'elle
   prouve au passage que le manifeste est du TOML valide.
2. `templates.rs` — comparer des `Path` plutôt que des `String`.

## Preuve

Non reproductible en local : ni runner ni émulation Windows. La non-régression se prouve
sur macOS, le correctif se prouve en CI — ce pour quoi la matrice existe.

La portée est close, elle n'est pas supposée : le job Windows exécute tous les tests non
`ignored`, et exactement deux avaient échoué.
