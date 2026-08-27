# M2 — Fragment `mail` : gabarits et rendu

Les gabarits sont **déposés dans le projet** et rendus par lui à l'exécution. Ils ne se
confondent pas avec les templates jinja du CLI, qui produisent le fragment lui-même : deux
étages de rendu, et deux jeux de délimiteurs. Le CLI lit `{@ … @}`, les gabarits de
courriel `{{ … }}` — un gabarit de message traverse donc le rendu du CLI intact, à
condition de n'employer aucun bloc `{% … %}`, que les deux étages se partagent.

1. `minijinja 2.24` déclarée par le manifeste de fragment, avec la feature `loader` :
   `path_loader` n'est pas dans les défauts de la crate.
2. `src/mail/gabarit.rs` — `Gabarits`, un `Environment` chargé depuis un répertoire, mis
   en `Arc` parce qu'`AppState` se clone à chaque requête. `path_loader` refuse d'ouvrir
   un chemin absolu ou remontant : un nom de gabarit venu d'une entrée utilisateur ne sort
   pas du répertoire.
3. `MailConfig` gagne `gabarits`, le répertoire, `templates/mail` par défaut.
4. `templates/mail/bienvenue.html` déposé dans le projet, à lire et à modifier.
5. `Mailer` gagne `envoyer_gabarit` et `envoyer_detache`. Ce dernier `tokio::spawn` sans
   file ni réessai — ce que le jalon peut tenir sans mentir (§2.5) — et journalise l'échec.
6. **TDD** : les trois tests de `src/mail/tests.rs`, écrits d'abord, vus rouges par
   `cargo test` dans un projet fraîchement généré.
   - Le troisième se prouve **sans serveur SMTP** : un `TcpListener` de boucle locale qui
     accepte la connexion et ne répond jamais. Un envoi attendu y resterait suspendu ;
     le test mesure que l'appel rend la main, puis attend que la connexion arrive quand
     même — sans quoi une fonction au corps vide passerait.
7. **Morsures**, une par ligne `✓` : contexte ignoré au rendu ; message d'erreur réduit à
   celui de minijinja, qui ne nomme que le gabarit et jamais le fichier ; attente ajoutée
   avant le `spawn`.
