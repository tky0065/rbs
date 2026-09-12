# Client TypeScript : `Option<Struct>` rend `null | T` — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Un DTO portant un objet imbriqué optionnel garde son type côté client : `oneOf: [{type:"null"}, {$ref}]` (la forme qu'utoipa produit pour `Option<Struct>`) se rend `null | T`, et non `unknown | T`, qui s'absorbe en `unknown`.

**Architecture:** `document::parse_schema` rend déjà ce `oneOf` en `Schema::Union([Primitive{kind:"null"}, Ref])` ; seul `ts::primitif` ignore `"null"` et tombe dans `_ => "unknown"`. Une branche `"null" => "null"` suffit — `type_de` joint les variantes d'une union dans l'ordre du document, et `Schema::Inconnu` reste le seul porteur légitime d'`unknown`.

**Tech Stack:** Rust, tests unitaires de `crates/rbs-cli/src/client/ts.rs` (`cargo test -p rbs-cli --lib client::ts`).

**Backlog :** IMPROVE.md, tâche 17.

## Global Constraints

- Aucun autre `unknown` de `ts.rs` n'est concerné : `Schema::Inconnu` (schéma sans `type`) et `Record<string, unknown>` (objet sans propriétés) sont des rendus voulus et testés.
- Commit Conventional, sujet en français, sans identifiant de tâche ni ligne d'attribution.

---

### Task 1: rendre `null` la variante nulle d'une union

**Files:**
- Modify: `crates/rbs-cli/src/client/ts.rs` (`primitif`, tests)

- [x] **Step 1: Test rouge** — `an_optional_struct_is_a_union_of_null_and_its_reference` : `type_du_champ(r#"{"oneOf":[{"type":"null"},{"$ref":"#/components/schemas/S"}]}"#)` doit rendre `null | S`. Attendu rouge : `unknown | S`. — vu rouge : `left: "unknown | S"`, `right: "null | S"`
- [x] **Step 2: Fix** — `"null" => "null"` dans `primitif`.
- [x] **Step 3: Vert** — `cargo test -p rbs-cli --lib client::ts`, puis `--lib` complet, clippy, fmt. — `client::ts` 30 passés, `--lib` 1158 passés, clippy et fmt propres
