# T2 — Pages de documentation FR et EN

**Conception.** Les trois critères ne nomment que `jobs` et `mail`, mais le jalon a livré
quatre choses : les seeds, `rbs dev`, les jobs, et le choix du moteur. `rbs dev` et
`--database` n'apparaissent nulle part dans les dix-huit pages du site, `rbs seed` une
seule fois dans `getting-started.md`. Le périmètre retenu est le jalon entier — sortir
« Confort » avec sa commande de développement absente de la documentation serait un trou
qu'aucun critère de `T4` ne mesure.

**Un exemple par page**, sans exception sur les dix pages qui en citent un aujourd'hui.
C'est ce qui décide de la page de `mail` : elle migre de `file-drop` vers
`newsletter-queue`, seul moyen de tenir ensemble « aucun extrait non issu de
`newsletter-queue` » et « montre le passage à un job ». Elle y gagne les deux moitiés du
contraste, l'exemple ayant délibérément conservé `send_detached` sous une permission qui
ne vaut que pour lui.

**Ce que mesure la parité** (`docs/scripts/parite.mjs`) : la charpente des titres, la
langue et la méta des blocs de code, le type des encarts, la cible des liens relatifs. Le
texte traduit n'est jamais comparé. Les deux versions d'une page ont donc la même
structure, à la ligne près, et citent les mêmes régions du même fichier.

## Pages

| Page | Position | Extraits |
|---|---|---|
| `guides/jobs.md` | 11 | trait `Job`, le job, le registre, l'enfilage transactionnel, la configuration, le worker |
| `guides/seeds.md` | 12 | le seed d'une entité, l'ancre et son binaire |
| `guides/mail.md` | 9, migrée | `send_detached` et le job qui le remplace |
| `cli/seed.md` | 6 | le refus sous `production` |
| `cli/dev.md` | 7 | — |
| `cli/new.md` | 1, complétée | la section `--database` |

## Étapes

1. Poser dans `examples/newsletter-queue` les régions que les pages citeront et qui
   n'existent pas encore — les marqueurs sortent de la comparaison de dérive, mais les
   fichiers déjà édités à la main en portent seuls la trace.
2. Écrire l'anglais, puis le français à structure identique, page par page.
3. Preuves : `node scripts/parite.mjs` ; un `grep` établissant qu'aucun bloc `file=` ne
   cite un autre exemple que `newsletter-queue` sur les pages écrites ; `npm run build`,
   qui fait tomber une région introuvable ; et la lecture de la page de `mail` par le user,
   son troisième critère étant de nature éditoriale.
