---
sidebar_position: 2.6
title: rbs routes
---

# `rbs routes`

Liste les routes du projet — méthode, chemin, `operation_id` et garde — en une commande,
en tableau pour un humain ou en JSON pour un script ou un agent.

:::note
rbs parle français dans ses écrans d'aide et dans ses sorties. Tous les blocs de terminal
de cette page sont verbatim, capturés en lançant la commande.
:::

## Synopsis

```text
$ rbs routes --help
Liste les routes du projet : méthode, chemin, operation_id et garde

Utilisation : rbs routes [OPTIONS]

Options :
      --json     Rend les routes en JSON sur la sortie standard, pour un script ou un agent
  -h, --help     Affiche l'aide
  -V, --version  Affiche la version
```

## Le tableau

Sur un projet créé par `rbs new routes-api --with auth` :

```text
$ rbs routes
MÉTHODE  CHEMIN                     OPERATION_ID              GARDE
POST     /auth/change-password      auth_change_password      bearer
POST     /auth/forgot-password      auth_forgot_password      public
POST     /auth/login                auth_login                public
POST     /auth/logout               auth_logout               public
GET      /auth/me                   auth_me                   bearer
POST     /auth/refresh              auth_refresh              public
POST     /auth/register             auth_register             public
POST     /auth/resend-verification  auth_resend_verification  public
POST     /auth/reset-password       auth_reset_password       public
GET      /auth/sessions             auth_list_sessions        bearer
DELETE   /auth/sessions             auth_revoke_sessions      bearer
DELETE   /auth/sessions/{id}        auth_revoke_session       bearer
POST     /auth/verify-email         auth_verify_email         public
GET      /health                    health                    public
GET      /health/live               health_live               public
```

Les lignes sont triées par chemin, puis par méthode dans l'ordre GET, POST, PUT, PATCH,
DELETE ; tout autre verbe vient après. Une opération sans `operation_id` affiche `-` dans
cette colonne.

`GARDE` vaut `bearer` quand l'opération déclare une `security` non vide — ou, quand elle
n'en déclare aucune, quand le document en déclare une globale — et `public` sinon. Une
`security` vide posée explicitement sur une opération la rend publique même sous une
exigence globale, comme OpenAPI le prévoit. Un handler engendré sous
[`auth`](../guides/auth.md) porte `security(("bearer" = []))` dans son annotation, et c'est
ce que cette colonne lit.

## `--json`

Un tableau d'objets à quatre clés, toujours présentes — `operation_id` vaut `null` quand
l'opération n'en a pas. C'est la seule chose sur la sortie standard : la compilation du
projet part sur la sortie d'erreur.

```text
$ rbs routes --json
[
  {
    "methode": "POST",
    "chemin": "/auth/change-password",
    "operation_id": "auth_change_password",
    "garde": "bearer"
  },
  {
    "methode": "POST",
    "chemin": "/auth/forgot-password",
    "operation_id": "auth_forgot_password",
    "garde": "public"
  },
  …
]
```

Le bloc est coupé après deux entrées ; la commande les imprime toutes.

## D'où viennent les routes

Du document OpenAPI, non du router : le document porte, pour chaque opération, son
`operation_id` et l'exigence de jeton que son annotation déclare — ce que le router ne dit
pas. Le document s'obtient comme [`rbs openapi export`](./openapi.md) et
[`rbs generate client`](./client.md) l'obtiennent, en lançant le binaire `openapi` du
projet : une route montée sans `#[utoipa::path]` n'y figure donc pas. Elle manque aussi au
document, et à tout client engendré depuis lui.

Les échecs sont ceux de `rbs openapi export`, mot pour mot. Sous `--json`, l'erreur et son
remède partent tous deux sur la sortie d'erreur. Le code de sortie est 1 pour chaque refus
qui y est décrit, une faute du projet, et 2 hors d'un projet — voir les [codes de
sortie](./doctor.md#codes-de-sortie).
