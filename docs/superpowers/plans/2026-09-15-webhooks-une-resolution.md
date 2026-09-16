# Une seule résolution DNS par livraison webhook

Design validé en conversation le 2026-09-15 (chemin borné, pas de spec).

1. Tests rouges, dans `templates/features/webhooks/tests.rs.jinja` et `examples/event-hub` :
   un client en politique stricte vers `localhost` rend une erreur où `refusal_in` trouve
   `Refusal::PrivateHost` ; un port fermé en `development` n'en porte aucun. Le test du
   résolveur perd sa moitié `Policy::resolve`.
2. `target.rs.jinja` : supprimer `Policy::resolve`, ajouter `refusal_in`, reprendre le
   commentaire de `Resolver`.
3. `delivery.rs.jinja` : `post` envoie après `check` ; l'erreur d'envoi qui porte un refus
   devient `PostError::Blocked`.
4. Guide webhooks, anglais et français : une résolution, dans le résolveur du client.
5. Vérifier : tests hors base d'`event-hub`, `integration_examples`, fmt, clippy, build de la
   doc, `integration_webhooks` sous Docker.
