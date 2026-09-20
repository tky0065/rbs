# Les raccourcis de hello-crud, et rien d'autre : ce fichier n'appelle que cargo,
# npm et docker compose. Le projet se clone et se lance sur une machine qui n'a pas rbs.
.DEFAULT_GOAL := help

# Ce que `make dev` mène de front, chaque commande terminée par `&`. Le squelette n'en
# lance qu'une ; un fragment qui apporte un exécutable de plus ajoute la sienne à la
# suite, et la recette les attend toutes.
DEV = cargo run &

help: ## affiche cette liste
	@awk 'BEGIN { FS = ":.*## "; print "Raccourcis de hello-crud — make <cible>\n" } /^[a-z][a-z0-9-]*:.*## / { printf "  %-12s %s\n", $$1, $$2 }' $(MAKEFILE_LIST)

# Les deux processus — le binaire, et ce qu'un fragment a ajouté à `DEV` — vivent dans un
# même groupe : `kill 0` les emporte ensemble au premier Ctrl-C, et `wait` tient la
# recette ouverte tant qu'il en reste un.
dev: ## lance tout ce que le projet porte, jusqu'à Ctrl-C
	@trap 'kill 0' INT TERM EXIT; \
	$(DEV) \
	wait

back: ## lance l'API seule
	cargo run

build: ## compile en mode optimisé
	cargo build --release

# Les tests qui joignent la base sont `#[ignore]` : ajoutez `-- --include-ignored` une
# fois `make up` passé, comme le fait la CI.
test: ## lance les tests
	cargo test --workspace

lint: ## passe clippy, avertissements compris
	cargo clippy --workspace --all-targets -- -D warnings

fmt: ## reformate le code
	cargo fmt --all

migrate: ## applique les migrations en attente
	cargo run -p migration -- up

seed: ## insère les données de démonstration
	cargo run --bin seed

up: ## démarre les services de docker-compose.yml
	docker compose up -d

down: ## arrête ces mêmes services
	docker compose down

openapi: ## écrit le document OpenAPI sur la sortie standard
	cargo run --bin openapi

clean: ## efface les artefacts de compilation
	cargo clean

# Les raccourcis qu'ajoutent les fragments s'écrivent entre les deux balises ci-dessous :
# `rbs add frontend` y pose `front`, `rbs add docker` y pose `image`. Un `##` en fin de
# ligne de cible la fait paraître dans `make help`, et chaque fragment déclare les
# siennes en `.PHONY`.
.PHONY: help dev back build test lint fmt migrate seed up down openapi clean
# <rbs:make>
# </rbs:make>
