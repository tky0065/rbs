# X2 — Les notes de migration embarquées

**Conception.** Un fichier par saut de version, embarqué par `include_dir` comme les
templates, et affiché par `upgrade` après l'application du plan.

**Leur longueur mesure la qualité du gel.** C'est la vertu de second ordre que la conception
§2.5 relève : des notes longues diraient que l'API a mal été figée. Celle du saut
0.4.0 → 1.0.0 doit donc être courte — la seule rupture commise est le bras `_ =>` que
`#[non_exhaustive]` impose au filtrage exhaustif des sept enums publics, et la construction
littérale interdite sur les structs qui n'avaient pas déjà un champ privé.

**Les notes vivent sous `crates/rbs-cli/`**, comme les templates : `cargo package` n'emporte
aucun fichier extérieur au paquet, et `include = [...]` ne lève pas cette règle. À la racine,
elles compileraient en local et manqueraient au paquet publié.

**Les deuxième et troisième critères ne se contredisent qu'en apparence.** Un saut sans note
rend un message neutre et un code 0 — toutes les versions n'ont pas de rupture à raconter.
Le test de complétude, lui, ne porte que sur **un** saut : celui qui va de la dernière
version publiée à celle que le binaire porte. Sans lui, le catalogue pourrirait en silence :
on publierait une version en ayant oublié d'écrire ce qu'elle casse.

**Ce test passe à vide aujourd'hui, et c'est voulu.** La dernière publiée et la version du
binaire valent toutes deux 0.4.0 : aucun saut, rien à exiger. Il mordra le jour où `Y2`
monte le workspace à 1.0.0 — exactement quand il le faut, et la note écrite ici l'attendra.

**Il ne faut donc pas se contenter de le voir passer.** Un test vert qui n'a rien examiné ne
prouve rien. La version se paramètre — motif employé trois fois dans ce dépôt : `NOYAU_PUBLIE`
dans `doctor/versions.rs`, `plan_for_with` dans `upgrade.rs`, `check_with` dans `doctor`. Le
test doit être exercé sur une version fictive dont la note manque, et échouer.

## Étapes

1. Créer `crates/rbs-cli/migrations/` (ou le nom retenu) et y écrire la note du saut
   0.4.0 → 1.0.0 : courte, disant la rupture réelle du gel.
2. L'embarquer par `include_dir`, sur le modèle de `templates.rs`.
3. Brancher l'affichage dans `upgrade` : après l'application du plan, la note du saut si
   elle existe ; sinon un message neutre et un code 0, jamais une erreur.
4. Écrire le test de complétude, **paramétré sur la version**, et le prouver en l'exerçant
   sur un saut dont la note manque : il doit échouer, et nommer la note attendue.
5. Preuves : les trois critères joués séparément ; pour le premier, la sortie réelle
   d'`upgrade` sur un saut vers 1.0.0, capturée sur le binaire.
