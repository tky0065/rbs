---
sidebar_position: 1
title: Logs
---

# Logs

rbs porte deux formateurs de logs : `pretty`, fait pour être lu par un humain pendant le
développement, et `json`, fait pour être analysé par un collecteur. `RBS_LOG_FORMAT`
choisit entre les deux — `pretty` est la valeur par défaut — et `RUST_LOG` filtre, comme
partout ailleurs dans l'écosystème Rust.

## Le formateur `pretty`

`tracing-subscriber` a son propre formateur. rbs ne l'utilise pas : il affiche plus que ce
qu'un développeur lit. `pretty` rend un événement par ligne, avec des colonnes qui ne
bougent pas d'une ligne à l'autre, pour que l'œil trouve le niveau et le message sans
avoir à balayer.

![Sortie du formateur pretty sur les cinq niveaux](/img/logs-pretty.png)

De gauche à droite : un horodatage court, le niveau, la cible, le message, puis les champs
de l'événement — suivis de ceux du span englobant, comme sur la ligne `ERROR` ci-dessus,
qui porte le `request_id` du span dans lequel elle a été émise.

Les niveaux sont colorés : `TRACE` gris, `DEBUG` bleu, `INFO` vert, `WARN` jaune, `ERROR`
rouge. Les champs et la cible sont atténués, pour que le message garde l'attention du
lecteur. **Les couleurs disparaissent quand la sortie n'est pas un terminal** : un fichier
de log redirigé ne contient aucune séquence d'échappement.

## Émettre des événements

Rien du formateur n'est spécifique à rbs : l'émission passe par les macros de `tracing`.

```rust file=crates/rbs-core/examples/logs_pretty.rs region=niveaux
```

## L'installer à la main

`rbs_core::logs::init()` lit `RBS_LOG_FORMAT` et pose le bon formateur — un projet généré
l'appelle au démarrage et n'a besoin de rien d'autre. Pour poser `pretty` sans condition,
sur un harnais de test ou un binaire ponctuel, construisez l'abonné vous-même :

```rust file=crates/rbs-core/examples/logs_pretty.rs region=installation
```

Lancez cet exemple pour juger du rendu sur votre propre terminal :

```bash
cargo run -p rbs-core --example logs_pretty
```

L'image ci-dessus est régénérée depuis cette même commande, jamais retouchée à la main :

```bash
python3 docs/scripts/capture_logs_pretty.py
```
