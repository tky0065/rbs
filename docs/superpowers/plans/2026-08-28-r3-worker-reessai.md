# R3 — Le worker : réservation, réessai, échec définitif

**Conception.** Tout le dépilage tient dans une instruction et dans une fonction,
`reserver_prochain_job` : un `UPDATE … WHERE id = (SELECT … FOR UPDATE SKIP LOCKED LIMIT 1)
RETURNING …`. Réserver et marquer sont le même acte, donc deux workers ne peuvent pas se
partager une ligne, et `S3` n'aura qu'un corps de fonction à trois branches à écrire.

Le reste du worker ne connaît que des `Model` : boucle, dispatch par `kind` dans un
registre, puis `done`, ou `pending` avec `available_at` repoussé, ou `failed` quand
`attempts` atteint `max_attempts`. Les deux valeurs viennent de `[jobs]`.

## Étapes

1. Trois tests livrés au projet : deux workers concurrents ne se partagent pas un job ;
   un job qui échoue est réessayé ; il devient `failed` après N tentatives.
2. Un test du dépôt qui `grep`e le fragment : `FOR UPDATE SKIP LOCKED` n'y paraît qu'une
   fois, et dans le seul `queue.rs`.
3. Implémenter `reserver_prochain_job`, `mark_done`, `retry_or_fail`, puis la boucle.
4. Morsure : retirer `SKIP LOCKED` — le test de concurrence doit tomber, et lui seul.
