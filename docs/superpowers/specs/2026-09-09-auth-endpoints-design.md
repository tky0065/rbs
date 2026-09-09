# Huit routes de plus au fragment `auth`

Date : 2026-09-09
Portée : `crates/rbs-cli/templates/features/auth/`, `crates/rbs-cli/templates/features/rate-limit/feature.toml`, `examples/blog-auth/`, six pages de `docs/`.
Hors portée : `rbs-core`, qui ne change pas.

## Le problème

Le fragment `auth` couvre cinq routes — `register`, `login`, `refresh`, `logout`, `me` —
et deux tables. Un projet engendré sait donc ouvrir un compte et une session, mais rien de
ce qui arrive ensuite : un mot de passe oublié n'a aucun chemin de retour, un mot de passe
connu ne se change pas, une adresse déclarée à l'inscription n'est jamais vérifiée, et un
titulaire ne voit pas les sessions ouvertes en son nom. Ces quatre manques se comblent
aujourd'hui à la main, dans chaque projet, sur un module dont les invariants de sécurité
sont précisément ce qu'on ne veut pas voir réécrit à chaque fois.

## Les routes

Huit routes s'ajoutent aux cinq existantes. Aucune route existante ne change de contrat.

| Route | Auth | Succès | Refus |
|---|---|---|---|
| `POST /auth/change-password` | Bearer | 200 + `TokenPair` | 403, 422 |
| `POST /auth/forgot-password` | — | 202 | 422 |
| `POST /auth/reset-password` | — | 204 | 401, 422 |
| `POST /auth/verify-email` | — | 204 | 401 |
| `POST /auth/resend-verification` | — | 202 | 422 |
| `GET /auth/sessions` | Bearer | 200 + `Vec<SessionResponse>` | 401 |
| `DELETE /auth/sessions/{id}` | Bearer | 204 | 401, 404 |
| `DELETE /auth/sessions` | Bearer | 204 | 401 |

### `change-password` rend une paire neuve

Le changement révoque **toutes** les sessions du compte, celle de l'appelant comprise :
la requête porte un jeton d'accès, et rien ne le relie à la ligne `refresh_tokens` qui l'a
émis — la session courante ne peut pas être épargnée faute d'être identifiable. Plutôt que
de déconnecter quelqu'un qui vient de faire la bonne chose, le service révoque puis
appelle `issue()`. Le client repart avec une paire valide ; les autres appareils tombent.

Le mot de passe courant faux rend **403 et non 401**. L'appelant est identifié, son Bearer
est bon : un 401 lui dirait que son jeton est mort et déclencherait un `refresh` inutile.

### `forgot-password` et `resend-verification` rendent 202, toujours

Y compris pour une adresse qu'aucun compte ne porte. C'est l'invariant qui gouverne déjà
`login`, où le hash témoin fait payer un Argon2 à une adresse inconnue : distinguer les
deux cas énumère les comptes.

Corollaire non évident : **l'envoi SMTP part dans un `tokio::spawn` détaché**. Attendu dans
le chemin de réponse, il rendrait par le seul temps de réponse ce que le code de statut
refuse de dire — quelques centaines de millisecondes séparent une adresse inscrite d'une
autre. L'échec d'envoi est journalisé en `error` et n'atteint pas le client, qui a déjà sa
202 : le statut décrit exactement ce qui se passe, une demande acceptée dont l'exécution
suit.

### `register` envoie désormais le courriel de vérification

Son contrat ne change pas : toujours 201, toujours `UserResponse` — avec un champ
`email_verified_at` de plus, nul à l'inscription. L'envoi suit la même règle que
`resend-verification`, détaché.

### Les sessions

`GET /auth/sessions` liste les sessions **ouvertes** du compte : `id`, `created_at`,
`expires_at`. Jamais `token_hash` — la vue publique d'une session n'a aucune raison de
porter de quoi la présenter.

`DELETE /auth/sessions/{id}` révoque une session nommée, et rend 404 si l'identifiant ne
désigne aucune session du compte appelant. Le filtre porte sur `(id, user_id)` dans la même
requête : lire la ligne puis comparer le propriétaire laisserait la révocation d'autrui à
portée d'une course.

`DELETE /auth/sessions` révoque tout, `revoke_sessions_of(user_id)`. C'est le geste du
titulaire qui doute — il se reconnecte ensuite, sur l'appareil qu'il a en main.

## Le schéma

Une table de plus dans la migration `create_auth_tables`. Cette migration **crée** les
tables et n'en altère aucune : un projet déjà engendré et migré n'a rien à rattraper, et
`rbs upgrade` — qui ne retouche jamais le code d'une feature installée — n'a pas à
connaître ce changement.

```
one_time_tokens
  id           uuid pk          uuidv7 posé par ActiveModelBehavior::new
  user_id      uuid not null    fk users, on delete cascade
  token_hash   string not null  indexé ; l'empreinte, jamais le jeton
  purpose      string not null  "password_reset" | "email_verification"
  expires_at   timestamptz not null
  consumed_at  timestamptz null nul tant que le jeton vit
  created_at   timestamptz not null default now()
  updated_at   timestamptz not null default now()
```

Et une colonne sur `users` : `email_verified_at timestamptz null`.

`purpose` est un `DeriveActiveEnum` stocké en texte, comme `Role` : un troisième usage
s'ajoute dans `model.rs` sans migration.

`consumed_at` et non `revoked_at`, qui nomme la colonne homologue de `refresh_tokens` : ce
n'est pas le même verbe. Un jeton de rafraîchissement est retiré ; celui-ci s'épuise en
servant. La ligne survit à son usage — c'est elle qui distingue un jeton déjà joué d'un
jeton jamais émis.

Un index sur `token_hash`, pour la même raison que sur `refresh_tokens` : chaque
consommation cherche une ligne par cette colonne.

### La consommation est atomique

`consume_one_time_token` reprend le patron de `repository::consume` :

```
UPDATE one_time_tokens
   SET consumed_at = now()
 WHERE id = ? AND consumed_at IS NULL AND expires_at > now()
```

et c'est `rows_affected == 1` qui autorise la suite. La ligne est d'abord retrouvée par
`find_one_time_token(fingerprint, purpose)` : **l'usage fait partie de la recherche**, sans
quoi un jeton de vérification vaudrait comme jeton de réinitialisation. **La péremption,
elle, est dans la condition de l'`UPDATE` et non dans la lecture qui le précède** : sinon
deux `reset-password` concurrents portant le même jeton franchissent tous deux la lecture,
et posent chacun leur mot de passe — le second gagnant sans que le premier le sache.

Jeton inconnu, périmé ou déjà consommé rendent la même `Error::Unauthorized`. Les
distinguer renseignerait sur l'état des demandes en cours.

### Une émission invalide les jetons vivants du même usage

Avant d'insérer, `invalidate_pending(user_id, purpose)` pose `consumed_at = now()` sur les
jetons non consommés du même couple. Sans cela, l'utilisateur qui redemande un lien parce
que le premier est parti dans une boîte qu'il ne contrôle plus laisse ce premier lien
valide jusqu'à son terme.

### Les lignes mortes

Les jetons consommés et périmés s'accumulent sans borne. Le repository fournit
`purge_expired()`, non montée, sous un `#[allow(dead_code)]` documenté — le patron de
`RequireRole` dans `guard.rs`, où le commentaire dit à quelle condition la ligne se retire.
Le fragment ne branche pas de tâche périodique : `scheduler` n'est pas requis par `auth`, et
le conditionner sur sa présence ajouterait une fragilité d'ordre d'installation là où une
fonction prête à appeler suffit.

## Ce que le fragment installe

```toml
requires = ["rate-limit", "mail"]
```

`mail` devient une dépendance dure. Le coût est réel et assumé : `rbs add auth` installe
désormais `lettre`, la section `[mail]`, Mailpit dans `docker-compose.yml`, `templates/mail/`
et `RBS_MAIL__SMTP_PASSWORD` dans `.env.example` — y compris pour un projet qui ne voulait
qu'un login. En échange, le reset et la vérification marchent à la sortie de la boîte, sans
qu'aucun point d'extension reste à remplir.

Trois clés dans la section `[auth]` :

```toml
reset_ttl_secs = 3600           # une heure : le lien de réinitialisation est urgent
verification_ttl_secs = 86400   # un jour : la vérification ne l'est pas
app_url = "http://localhost:3000"
```

`app_url` pointe vers l'application du client et non vers le serveur : rbs engendre une
API, et le lien du courriel mène à l'écran qui postera le jeton. Les gabarits rendent
`{{ app_url }}/reset-password?token=…` et `{{ app_url }}/verify-email?token=…`.

Deux gabarits déposés dans le projet, à côté du `bienvenue.html` que `mail` y met déjà :
`templates/mail/reinitialisation.html` et `templates/mail/verification.html`. Déposés et non
embarqués — c'est du texte que son auteur relit et récrit.

Deux limites strictes de plus dans la configuration de `rate-limit`, dont le fichier nomme
déjà `/auth/login` :

```toml
{ path = "/auth/forgot-password", limit = 3, window_secs = 3600 },
{ path = "/auth/resend-verification", limit = 3, window_secs = 3600 },
```

Trois par heure et par adresse cliente : ces deux routes envoient un courriel à une adresse
que l'appelant choisit. Sans borne, elles font du projet un relais de harcèlement.

## La découpe des fichiers

À huit routes de plus, `service.rs` passerait de 177 à ~360 lignes et `controller.rs` de
107 à ~280 — au-delà des ~200 lignes que le projet donne comme signal, et sans rapport avec
la lisibilité qui fait tenir ce fragment.

**Chaque couche devient un répertoire ; les couches ne bougent pas.**
`controller → service → repository → model` reste vrai à la lettre : c'est le nombre de
fichiers par couche qui change, pas leur rôle ni le sens de la dépendance.

```
src/auth/
  mod.rs        routes() et impl HasAuth
  model.rs      Role, TokenPurpose, user, refresh_token, one_time_token
  dto.rs
  guard.rs      RequireRole, VerifiedIdentity
  repository/   mod.rs · user.rs · refresh_token.rs · one_time_token.rs
  service/      mod.rs · session.rs · password.rs · verification.rs
  controller/   mod.rs · session.rs · password.rs · verification.rs
  tests/        mod.rs · session.rs · password.rs · verification.rs
```

`service/mod.rs` garde `issue()` et `profile()`, que les trois autres partagent, et
réexporte. `repository/mod.rs` réexporte `Model` et `ADRESSE_PRISE`, que le service lit
déjà par cette porte. Le fragment passe de 8 à 19 entrées `[[files]]`, et l'ancre `openapi`
de 5 à 13 lignes.

## La garde de vérification

`login` ne change pas : un compte dont l'adresse n'est pas vérifiée se connecte. La
décision appartient au projet, et `guard.rs` lui donne de quoi la prendre —
`VerifiedIdentity`, un extracteur qui rejette en 403 une identité dont
`email_verified_at` est nul, à poser sur les routes que son auteur juge sensibles.

L'extracteur relit la base : le jeton d'accès porte `sub` et `role`, pas l'état de
vérification, et l'y mettre figerait cet état pour la durée du jeton — une adresse vérifiée
resterait non vérifiée un quart d'heure.

Comme `RequireRole`, il n'est appelé par aucune route du fragment et porte donc le même
`#[allow(dead_code)]` documenté.

## La testabilité

La base ne garde que l'empreinte. Un test qui poste `forgot-password` ne peut donc pas
enchaîner sur `reset-password` : il n'a pas le jeton en clair, et aucune lecture ne le lui
rendra.

**`service::request_password_reset()` et `service::request_verification()` rendent le jeton
en clair.** Le contrôleur le passe au `Mailer`, puis le laisse tomber — il ne traverse
jamais la frontière HTTP. Les tests du projet engendré appellent la couche service et
tiennent le jeton, ce qui leur permet de dérouler les deux flux entiers sans qu'aucun SMTP
soit joignable : l'envoi détaché échoue en silence et n'échoue que dans le journal.

Ce que les tests couvrent, par lot :

- **Mot de passe** : changement nominal ; ancien mot de passe faux → 403 ; le changement
  révoque les sessions et la paire rendue est utilisable ; reset nominal ; jeton périmé,
  inconnu, déjà consommé → 401 ; une seconde demande invalide la première ;
  `forgot-password` sur une adresse inconnue → 202 sans ligne créée.
- **Vérification** : `register` crée un jeton ; vérification nominale posant
  `email_verified_at` ; jeton d'un autre usage refusé ; `VerifiedIdentity` accepte après
  vérification et rejette avant.
- **Sessions** : la liste ne montre que les sessions ouvertes du seul appelant ; la
  révocation nommée d'une session d'autrui rend 404 ; la révocation globale ferme tout.
- **Consommation concurrente** : deux `consume` simultanés du même jeton, un seul à `true`.

## L'onde de choc

1. `examples/blog-auth` se régénère et gagne tout le fragment `mail`. `integration_examples.rs`
   compare octet à octet : il reste rouge tant que l'exemple n'a pas suivi. La régénération
   se fait par diff entre deux générations, jamais par écrasement — l'exemple porte des
   éditions à la main que `examples/README.md` recense.
2. Six pages de documentation, bilingues par paires : `guides/auth.md`, `tutorials/auth.md`,
   `cli/add.md` et leurs trois miroirs sous `docs/i18n/fr/`. Toute transcription de sortie
   du CLI qu'elles citent est vérifiée par test.
3. Le plan qu'affiche `rbs add auth` annonce désormais l'installation de `mail`, ce qui
   change la transcription attendue par les tests d'intégration du CLI.
4. Le client TypeScript se déduit du document OpenAPI : les huit routes y arrivent sans
   qu'aucun générateur ne change.

`rbs-core` ne bouge pas. `Identity`, `Error::Forbidden`, `token::random`,
`token::fingerprint`, `hash::hash_password` et `hash::verify_password` couvrent les huit
routes. Rien à publier côté noyau, et la frontière noyau / généré reste où elle est : tout
ce qui s'ajoute est du code que le développeur lira et modifiera.

## Les lots

Quatre lots, dans cet ordre. Chacun compile et se teste seul.

1. **Socle** — la table `one_time_tokens`, la colonne `email_verified_at`, le repository de
   jetons avec sa consommation atomique, la découpe de `auth/` en répertoires par couche,
   l'ajout de `mail` à `requires` et les trois clés de configuration. Aucune route nouvelle : à la
   fin de ce lot le fragment fait exactement ce qu'il faisait, sur une structure qui porte
   la suite.
2. **Mot de passe** — `change-password`, `forgot-password`, `reset-password`, le gabarit de
   réinitialisation, les deux limites strictes.
3. **Vérification** — `verify-email`, `resend-verification`, l'envoi depuis `register`, la
   garde `VerifiedIdentity`, le gabarit de vérification.
4. **Sessions** — les trois routes de session.

La régénération de `examples/blog-auth` et les six pages de documentation se font au fil des
lots, et non à la fin : un exemple périmé fait échouer la suite dès le lot 1, qui change
déjà le manifeste du fragment.
