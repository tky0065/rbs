# `integration_mail` sous Mailpit

## Ce qui s'ajoute

- `templates/features/mail/tests.rs.jinja` : un test `#[ignore]` qui **envoie** par
  `Mailer::depuis_config()` puis `envoyer_gabarit("bienvenue.html", …)`.
- `crates/rbs-cli/tests/integration_mail.rs` : Mailpit en `GenericImage`, ses deux ports
  publiés, et la **relecture par l'API HTTP** — c'est là que vit l'assertion.

## Décisions

- **L'envoi est livré à l'utilisateur, l'assertion reste au dépôt.** L3 livre ses trois
  tests parce qu'un projet doté d'un cache mérite la suite qui le prouve contre son
  serveur. Ici l'inverse : un projet n'a aucune raison d'hériter d'un test qui interroge
  l'API d'un serveur de développement. Le fragment livre le parcours d'envoi, le dépôt
  vérifie ce qui est arrivé.
- **Un transport en mémoire ne prouverait rien.** Ce que le critère demande — le corps
  *rendu par le gabarit*, relu à l'autre bout — n'existe qu'après la sérialisation MIME,
  l'échange SMTP et le décodage par le serveur.
- **Le client HTTP est écrit à la main sur `TcpStream`, en HTTP/1.0.** Aucune dépendance
  de développement ajoutée, aucun binaire externe appelé. L'HTTP/1.0 est le point : il
  interdit le *chunked encoding* que le serveur Go de Mailpit emploierait sinon, et
  dispense de le décoder. Le corps se parse par `serde_json`, déjà en dev-dep.
- `RBS_MAIL__SMTP_HOST` et `RBS_MAIL__SMTP_PORT` surchargent la section `[mail]` : le
  conteneur reçoit ses ports au démarrage, que `config/default.toml` ne peut pas connaître.

## Les deux faux verts à fermer

1. `cargo test -- --ignored` sort en 0 **sans avoir filtré aucun test** : le nom du test
   d'envoi est cherché dans le journal, comme en `L3`.
2. Une boîte relue trop tôt, ou un reliquat, passerait une assertion de simple présence :
   le test exige **exactement un** message, et assert le corps rendu — « Bonjour Ada, » et
   le lien, chaînes que seul le gabarit produit.

## Ordre

1. Le test d'envoi du fragment, `integration_mail.rs`, lancés → échec.
2. Vert, puis clippy et rustfmt sur le dépôt et sur un projet réel.
3. Morsures : `envoyer_gabarit` remplacé par un `message` au corps littéral (le corps
   rendu doit tomber, seul) ; le sujet altéré ; l'appel d'envoi retiré du test du fragment
   (le compte de messages doit tomber).
