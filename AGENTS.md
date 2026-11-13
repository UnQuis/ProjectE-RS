# AGENTS.md — ProjectE-RS contributor guide

## Project identity and goal

ProjectE-RS is an independent, clean-room Rust implementation of Equivalent Exchange mechanics for Minecraft 26.3 on Horizon.

This repository is not a fork, port, or derivative of ProjectE's source code. Reimplement behavior from public, user-observable functionality and independently written specifications. Do not copy, translate, adapt, or mechanically reproduce ProjectE source code, assets, textures, models, namespaced implementation details, or decompiled code. Do not use leaked, private, or otherwise unauthorized source material. When behavior is unclear, document the uncertainty and implement an original, testable design.

The project is intended to become a complete mod with its own assets and models. The current native crate is an early gameplay implementation, not a finished content package.

## Non-negotiable repository rules

1. **Clean room means clean room.** Do not inspect or use ProjectE source/decompiled jars as implementation references. Do not ask another agent to do so. Use public gameplay behavior, this repository's own code, and the pinned Horizon API.
2. **Pin compatibility.** The exact Horizon and mixins-tool commits are declared in `engine/horizon-mod.toml` under `[native]`. Never silently target Horizon `main`, a newer API, or a different Minecraft version. Re-check the exact pinned source before using an API not already present in this repository.
3. **Do not invent Horizon APIs.** Verify type names, hook names, method signatures, ownership, thread/side constraints, and serialization behavior against the pinned source. Distinguish internal Horizon methods from methods exported to native mods through `Host`.
4. **Preserve server authority.** EMC balances, learned knowledge, machine state, and transmutation decisions are server-authoritative. Client-side hooks may display state but must not grant items, debit EMC, or mutate authoritative machine data.
5. **Fail explicitly.** Do not hide storage, serialization, parsing, or transaction errors with `unwrap_or_default()`, `let _ = ...`, silent fallback, or lossy casts in production paths. Handle errors deliberately and provide actionable diagnostics.
6. **No unsafe arithmetic.** Use checked arithmetic for EMC costs, inventory counts, machine energy, and conversions. Reject impossible operations before granting items or charging balances. Never use wrapping/saturating arithmetic as a substitute for validation.
7. **Keep changes scoped.** Do not reformat or rewrite unrelated modules. Avoid modifying files assigned to another contributor/agent. State exact changed files and why.
8. **No false verification claims.** Report the exact commands actually run and their results. Static inspection, a Python model, or reasoning is not a Rust test/build.

## Repository layout and responsibilities

The native crate is `engine/`; the root `engine/horizon-mod.toml` points to it with `[api] path = "engine"`.

Expected module boundaries:

- `engine/src/lib.rs`: Horizon `Hooks` implementation, registration, hook orchestration, and integration between domain modules. Keep domain algorithms out of this file where practical.
- `engine/src/emc.rs`: typed DataMap descriptor/registration, merged-value reads, datapack resource emission, checked cost conversion and display metadata. Default values live in `engine/data/projecte/data_maps/item/emc.json`.
- `engine/src/transaction.rs`: pure transaction planning and settlement. Validate before side effects; charge only for items actually granted.
- `engine/src/data.rs`: player EMC/knowledge persistence and attachment codecs. Treat attachment operations as fallible.
- Future modules should own focused domains (for example `machines.rs`, `energy.rs`, `knowledge.rs`, `recipes.rs`, and UI modules) rather than expanding `lib.rs indefinitely.

The module list above describes intended boundaries; do not assume every future module already exists.

## Horizon API contract

The project currently pins:

- Minecraft target: `26.3`
- Horizon revision: `1024f7b983d87a9616e1e51c709ba845f6017047`
- Native Hooks API at that revision: `411`

The authoritative reference is the Horizon source at that exact commit and the repository's `HORIZON_API.md` once that documentation change is merged. API level numbers are not interchangeable: the native hooks API version is distinct from `horizon_mods::api_level::LEVEL`.

### Hooks and Host

- Hooks are process-wide, `Send + Sync`, and receive `&self`; persistent mutable state must use deliberate interior mutability.
- Use the `Host` supplied to the current callback. Do not assume a global game singleton.
- Hooks can be re-entered. Never hold a lock across a Host call that may trigger another hook.
- Treat event structs as non-exhaustive. Do not construct them or rely on exhaustive matching.
- For cancellable hooks, return `Flow::Cancel` only when the mod has actually claimed the operation. For collection hooks, append rather than replace other mods' registrations.
- Verify whether a function is an exported `Host` method before calling it. A method implemented privately inside Horizon's `Game` is not automatically available to this crate.

### Data Maps: supported typed API

The pinned Horizon revision exports the typed, read-only Data Map API:

- Implement `DataMap` with an entry registry (`ItemKey` for EMC), a value type and a strict `decode(&Tag)`.
- Register the descriptor from `data_map_types` with `event.types.push(Emc.ty())`.
- Emit built-in defaults from `data_pack_files` under `data_maps/projecte:item/emc`. The corresponding datapack path is `data/projecte/data_maps/item/emc.json`.
- Resolve runtime values with `ItemKey::data(host, &Emc)`; do not read Horizon's internal map storage or parse resource files yourself.

Horizon owns pack priority, conditions, tag expansion, `replace`, `remove`, and atomic visibility after a successful world-data load or `/reload`. Preserve the result contract:

- `Ok(Some(value))`: a decoded value.
- `Ok(None)`: this entry has no value, including a value removed by a higher-priority pack.
- `DataMapError::UnknownType`: the map was not registered for this registry.
- `DataMapError::Undecodable`: the stored JSON value cannot be represented by the decoder.
- `DataMapError::Unavailable`: no server data maps are available.

Data Maps are server-side and are not synced to clients at this revision. A client-side tooltip must not fall back to built-in defaults: that could display a value different from the server's datapack. Keep EMC transactions and machine decisions server-authoritative, and fail closed on map errors.

### Energy capabilities

The pinned API includes:

- `ModEnergyBuffer` / `block_entity_energy` for block entities.
- `ItemEnergyBuffer` / `item_energy_buffer` for item stacks.

Prefer these native capabilities over duplicate ProjectE attachments/custom-data energy storage. Before migrating, inspect exact pinned signatures and ensure the machine tick and item-stack replacement semantics are respected. Do not keep two competing sources of truth.

### Attachments

Attachments are appropriate for mod-owned player knowledge and EMC balance when no native capability owns that state.

- `AttachmentType::get/set/remove` are fallible.
- Preserve and handle `AttachmentError`; do not silently turn failures into defaults or discard failed writes.
- Add codecs for persisted state. Enable sync only when a client feature requires it.
- Do not use attachments to duplicate native block/item energy capabilities.

## EMC and transmutation invariants

- An unknown item has no EMC unless it has an explicit value or a deliberately supported, documented rule.
- Default EMC values are explicit entries in `engine/data/projecte/data_maps/item/emc.json`; there is no suffix-based runtime fallback.
- A resolved Data Map value is authoritative. Never resurrect an entry removed by a datapack with a Rust-side fallback.
- A modded item has no EMC unless a datapack explicitly assigns a value or includes it in a tag used by `projecte:emc`.
- A zero EMC value is not a free purchase; it means the item is not transmutable unless a separate explicit policy says otherwise.
- Cost calculations must reject multiplication overflow and values that cannot fit the player's balance type.
- Validate balance, inventory room, learned knowledge, item identity, and requested count before side effects.
- If the inventory accepts fewer items than requested, charge only for the accepted count.
- Reject impossible or malformed grant results. Never debit EMC if no item was granted.
- Keep transaction planning and settlement pure where possible so they can be unit-tested without Horizon runtime state.

## Data, recipes, assets, and UI

- Data Pack files are for server data, recipes, tags, and other data-pack resources; they are not a substitute for textures, models, blockstates, or localization assets.
- Prefer declarative data and native capabilities where the pinned Horizon API supports them.
- Keep recipes, EMC values, machine configuration, and progression data separate from hook orchestration.
- Assets should be original project assets. The project owner intends to create original models/textures; do not substitute copied ProjectE assets.
- A generic chest menu is acceptable as an initial UI, but custom transmutation search/category/paging behavior needs a deliberately designed client screen and server-authoritative transaction path.

## Rust style and implementation expectations

- Use the existing edition and dependency setup; do not add dependencies without a concrete need.
- Prefer small pure functions and explicit domain types over large hook methods.
- Use meaningful error enums instead of stringly typed internal control flow.
- Validate external strings and ids at boundaries; normalize unqualified vanilla item ids only where the command contract explicitly permits it.
- Avoid unnecessary allocations in hot hooks and machine ticks.
- Keep comments focused on invariants and non-obvious Horizon behavior, not line-by-line narration.
- Tests should cover boundary conditions, invalid inputs, overflow, partial success, and precedence rules—not only happy paths.

## Build and verification workflow

The normal native build must run through the Horizon-enabled launcher and the pinned Horizon source:

```bash
horizon-launcher native-build .
```

Run the crate's unit tests in the same Horizon-enabled checkout/toolchain environment:

```bash
cargo test
```

The crate currently has a local absolute `horizon-mod-api` path in `engine/Cargo.toml`; the launcher stages the crate and rewrites that dependency for its generated workspace. A plain standalone Cargo invocation may therefore fail outside the staged Horizon environment. Do not "fix" this by guessing a new dependency path; coordinate a portable dependency strategy separately.

Before reporting completion:

1. Inspect `git diff --check` and `git status --short`.
2. Run the narrowest relevant unit tests, then `cargo test`.
3. Run `horizon-launcher native-build .` when the launcher environment is available.
4. If a command cannot run, state why and leave it marked unverified.
5. Include changed files, behavioral impact, tests run, and remaining limitations in the PR summary.

## Collaboration and Git workflow

- Check the current branch, working tree, open PRs, and recent commits before editing.
- Do not assume `master` contains the newest implementation: review the PR base/head relationship first.
- Keep independent work on separate branches. Do not edit the same files as another agent without explicit coordination.
- If assigned a narrow task, stay within its declared file scope.
- Prefer focused PRs with one architectural purpose. Explain dependencies between PRs and target the correct base branch.
- Never force-push or rewrite another contributor's branch without explicit authorization.
- Root-level contributor guidance may be committed directly to `master` when the repository owner requests it; implementation changes should normally use reviewable branches/PRs.

## Completion checklist

- [ ] Clean-room compliance maintained.
- [ ] Pinned Horizon API verified for every new API surface.
- [ ] Server-authoritative behavior preserved.
- [ ] Errors and arithmetic handled explicitly.
- [ ] Tests added for edge cases and regressions.
- [ ] Only intended files changed.
- [ ] Build/test claims match commands actually executed.
- [ ] PR description documents scope, behavior, verification, and limitations.
