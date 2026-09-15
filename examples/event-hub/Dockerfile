# syntax=docker/dockerfile:1
FROM rust:1.94-slim-trixie AS builder

# `utoipa-swagger-ui` télécharge son archive pendant la compilation, avec `curl`, que
# l'image `rust:slim` ne porte pas. Sans lui, le build échoue à la toute fin, une fois
# tout le reste compilé.
RUN apt-get update \
 && apt-get install -y --no-install-recommends curl \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY . .
# `--bins` construit l'API et le binaire de migration : le service `migrate` du compose
# tourne sur cette même image.
#
# Les montages de cache gardent le registre et `target/` d'un build à l'autre : un commit
# ne recompile plus sea-orm ni axum. Ils n'entrent pas dans la couche, d'où la copie des
# deux binaires hors de `target/` dans la même instruction. Le cache de `target/` porte le
# nom du projet : partagé, deux projets du même builder se prêteraient leur paquet
# `migration`, qui a partout le même identifiant.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,id=event-hub-target,target=/build/target \
    cargo build --release --workspace --bins \
 && mkdir -p /build/out \
 && cp target/release/event-hub target/release/migration /build/out/

FROM debian:trixie-slim AS runtime

RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/* \
 && useradd --create-home --uid 10001 api

COPY --from=builder /build/out/event-hub /usr/local/bin/
COPY --from=builder /build/out/migration /usr/local/bin/

WORKDIR /app
COPY config ./config
# `mail` lit `templates/mail` à l'exécution, par un chemin relatif à /app : sans cette
# couche, l'API journalise « envoi échoué » à chaque courriel et n'envoie rien. Le motif
# tient lieu de condition — BuildKit copie ce qu'il trouve et ne se plaint pas du reste —
# et la ligne vaut quel que soit l'ordre dans lequel `mail` et `docker` ont été posés,
# `rbs add mail` après coup compris.
COPY --from=builder /build/templates* ./templates/

USER api
EXPOSE 8080

# L'image ne porte ni curl ni wget : la sonde parle HTTP par le `/dev/tcp` de bash. Elle
# vise `/health/live`, qui n'interroge rien, et non `/health` : une base tombée rendrait
# sinon le conteneur malade, et un orchestrateur qui lit ce verdict le remplacerait en
# boucle sans rien réparer. Posée ici plutôt que dans le compose, elle vaut aussi pour un
# `docker run` ; le port est celui d'`EXPOSE`, et l'un ne bouge pas sans l'autre.
HEALTHCHECK --interval=10s --timeout=3s --start-period=10s --retries=3 \
  CMD ["bash", "-c", "exec 3<>/dev/tcp/127.0.0.1/8080 && printf 'GET /health/live HTTP/1.1\\r\\nHost: localhost\\r\\nConnection: close\\r\\n\\r\\n' >&3 && head -n1 <&3 | grep -q ' 200 '"]

# Les deux binaires sont dans le PATH, et rien n'est figé par un ENTRYPOINT : le
# `command:` du compose y substitue `migration up`.
CMD ["event-hub"]
