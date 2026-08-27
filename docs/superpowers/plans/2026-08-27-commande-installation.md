# Correction de la commande d'installation annoncée

## Constat

Les deux pages de démarrage annoncent qu'une fois la 0.1 publiée, l'installation se fera
par `cargo install rbs`. Cette commande n'existera jamais : le nom `rbs` est déjà pris sur
crates.io par la crate de sérialisation de rbatis, publiée en 4.8.4. Les noms `rbs-core`
et `rbs-cli` sont libres, eux — l'installation se fera donc par `cargo install rbs-cli`.

## Étapes

Corriger la phrase dans les deux langues, et dire *pourquoi* le paquet ne porte pas le nom
du binaire. Sans cette raison, la question « pourquoi pas simplement `cargo install rbs` ? »
se pose au lecteur — et c'est exactement le genre de question que le critère de sortie
interdit de faire naître.

Le README n'est pas concerné : il dit déjà « le paquet est `rbs-cli`, le binaire qu'il
installe est `rbs` ».

## Preuve

Le nom relevé sur l'API de crates.io, non supposé, et le site reconstruit sur les deux
locales.
