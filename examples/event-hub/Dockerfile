FROM rust:1-slim-trixie AS builder

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
RUN cargo build --release --workspace --bins

FROM debian:trixie-slim AS runtime

RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/* \
 && useradd --create-home --uid 10001 api

COPY --from=builder /build/target/release/event-hub /usr/local/bin/
COPY --from=builder /build/target/release/migration /usr/local/bin/

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

# Les deux binaires sont dans le PATH, et rien n'est figé par un ENTRYPOINT : le
# `command:` du compose y substitue `migration up`.
CMD ["event-hub"]
