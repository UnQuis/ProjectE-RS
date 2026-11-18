# AGENTS.md — ProjectE-RS contributor guide

## Project identity and goal

ProjectE-RS is a Rust implementation of Equivalent Exchange mechanics for Minecraft 26.3 on Horizon, targeting the Horizon native mod API.

The repository owner explicitly authorized (2026-11-17) using the MIT-licensed ProjectE source as an implementation reference for this port. The original checkout is in the neighboring `../ProjectE` directory; the upstream is `github.com/sinkillerj/ProjectE` (MIT). Refer to it for gameplay behavior, EMC values, recipes, and mechanics. Preserve MIT attribution for any copied or adapted code, assets, or data. Do not use leaked, private, or otherwise unauthorized source material. When behavior is unclear, document the uncertainty and implement a testable design.

The project is intended to become a complete mod with its own assets and models. The current native crate is an early gameplay implementation, not a finished content package.

## Non-negotiable repository rules

1. **Reference source is permitted.** The MIT-licensed ProjectE source may be used as an implementation reference, as authorized by the repository owner (2026-11-17). The original checkout is in the neighboring `../ProjectE` directory; upstream is `github.com/sinkillerj/ProjectE` (MIT). Do not use decompiled jars or leaked/private material. Preserve MIT attribution for any copied or adapted code, assets, or data.
2. **Pin compatibility.** The Horizon revision is declared in `horizon-mod.toml` (and mirrored in `compatibility.json`). Never silently target Horizon `main`, a newer API, or a different Minecraft version. Re-check the exact pinned SDK before using an API not already present in this repository.
3. **Do not invent Horizon APIs.** Verify type names, hook names, method signatures, ownership, thread/side constraints, and serialization behavior against the pinned source. Distinguish internal Horizon methods from methods exported to native mods through `Host`.
4. **Preserve server authority.** EMC balances, learned knowledge, machine state, and transmutation decisions are server-authoritative. Client-side hooks may display state but must not grant items, debit EMC, or mutate authoritative machine data.
5. **Fail explicitly.** Do not hide storage, serialization, parsing, or transaction errors with `unwrap_or_default()`, `let _ = ...`, silent fallback, or lossy casts in production paths. Handle errors deliberately and provide actionable diagnostics.
6. **No unsafe arithmetic.** Use checked arithmetic for EMC costs, inventory counts, machine energy, and conversions. Reject impossible operations before granting items or charging balances. Never use wrapping/saturating arithmetic as a substitute for validation.
7. **Keep changes scoped.** Do not reformat or rewrite unrelated modules. Avoid modifying files assigned to another contributor/agent. State exact changed files and why.
8. **No false verification claims.** Report the exact commands actually run and their results. Static inspection, a Python model, or reasoning is not a Rust test/build.

## Repository layout and responsibilities

The native crate is the repository root: `Cargo.toml` + `src/`. The launcher requires this layout (`horizon-mod.toml`, `Cargo.toml` and `src/` at the mod root). A `.horizon/sdk/api/` directory holds the pinned generated SDK and is gitignored.

Expected module boundaries:

- `src/lib.rs`: `mod_api::Hooks` implementation, registration, hook orchestration, and integration between domain modules. Keep domain algorithms out of this file where practical.
- `src/emc.rs`: EMC value resolution, explicit default values, cost conversion helpers, and display/fuel metadata. Keep the resolver deterministic and independent of player state.
- `src/transaction.rs`: pure transaction planning and settlement. Validate before side effects; charge only for items actually granted.
- `src/data.rs`: player EMC/knowledge persistence and attachment codecs. Treat attachment operations as fallible.
- `src/machines.rs`, `src/energy.rs`, `src/condenser.rs`, `src/deposit.rs`, `src/deposit_menu.rs`, `src/alchemical.rs`, `src/ui/*`: machines, native capabilities and UI. Use Parts and native capabilities rather than expanding `lib.rs` indefinitely.

The module list above describes intended boundaries; do not assume every future module already exists.

## Horizon API contract (updated 2026-11-17)

The old API (`horizon-client`, native hooks level 411, `native-hooks.json`, `horizon-mixin.toml`, absolute `horizon-client` path) is **retired**. The project now targets the generated SDK mod API.

The project pins:

- Minecraft target: `26.3`
- Horizon revision: `4dc6455d4a6773224792eb1f121a9b939dff3170` (SDK release `sdk-4dc6455d…`, `arismoko/horizon`)
- Mod API: `0.2` (`api = "0.2"`), package format 6, Rust 1.99.1
- Manifest: `horizon-mod.toml` (mod id/version live here, not Cargo). Compatibility metadata: `compatibility.json`.
- SDK source: `.horizon/sdk/api/` (gitignored), installed from the `api-source.tar.xz` asset of the pinned SDK release. The root `Cargo.toml` depends on `mod_api` via `.horizon/sdk/api/crates/horizon-mod-api`.
- Authoritative references: the pinned SDK's generated code in `.horizon/sdk/api/crates/`, and `https://github.com/arismoko/horizon/tree/<revision>/docs/modding`.

### Hooks, Hosts and Parts

- The crate root holds one `#[mod_api::horizon_mod] impl mod_api::Hooks for <Mod>`; the mod type implements `Default`. The macro generates `hooks()` and subscribes exactly the written methods — do not write a factory.
- Hook methods take a side-specific host: `&mut dyn ServerHost`, `&mut dyn ClientHost`, or `HostSide<'_>` to match on. A call on the wrong side does not compile. Handles' reads take a host; writes take a server host.
- Events are generated structs named `<Hook>Event` with public fields. Cancellable hooks return `Flow` and the event has a `cancelled` field; replacement hooks return `Option`; collection hooks append to a list field. `#[listener(priority = <i32>, receive_cancelled)]` controls ordering.
- Per-object behaviour belongs to Parts: `BlockPart`, `BlockEntityPart`, `ItemPart`, `MenuPart`, `FluidPart`, attached via `registrar` in `register()`. Global hooks must not be used for object-scoped moments.
- Hook methods take `&self` and must be `Send + Sync`. The process-wide mod list is installed once; one mod instance serves both sides, so keep mutable state behind interior mutability. Hooks are re-entrant — never hold a lock across a call that can fire another hook.

### Capabilities and data

- Use native capabilities through the Host/Registrar: energy buffers (`ModEnergyBuffer`, `ItemEnergyBuffer`), containers (`ModContainer`), fluids, attachments. Do not duplicate them with custom data.
- Attachments are the place for mod-owned player knowledge and EMC balance. `get/set/remove` are fallible — handle errors explicitly and add codecs for persisted state; sync only when a client feature needs it.
- `native-hooks.json` is retired: the catalog is generated from Rust source. Never hand-write or depend on a `native-hooks.json` file.
- Data maps / datapack resources still require verifying the exact generated Host read methods at the pinned revision before relying on runtime lookup. Preserve pack precedence, tag expansion and removals; do not silently substitute a hand-rolled parser.

## EMC and transmutation invariants

- An unknown item has no EMC unless it has an explicit value or a deliberately supported, documented rule.
- Never infer arbitrary modded item values from suffixes such as `_ingot`, `_ore`, `_log`, or `_wool`.
- Explicit values take precedence over fallback rules.
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
- Assets may be adapted from the MIT-licensed ProjectE source where needed for functional parity; retain MIT attribution and prefer original assets where practical.
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

The `mod_api` dependency is a relative path into the gitignored `.horizon/sdk/api/` SDK. If that directory is missing, restore it from the pinned `api-source.tar.xz` SDK release before running Cargo (`curl` the release asset and extract into `.horizon/sdk/api/`). Do not replace it with a guessed path or with the retired `horizon-client` crate. Builds through `horizon-launcher native-build .` must match the pinned Horizon revision in `horizon-mod.toml`.

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

- [ ] MIT attribution preserved for any adapted ProjectE source/assets.
- [ ] Pinned Horizon API verified for every new API surface.
- [ ] Server-authoritative behavior preserved.
- [ ] Errors and arithmetic handled explicitly.
- [ ] Tests added for edge cases and regressions.
- [ ] Only intended files changed.
- [ ] Build/test claims match commands actually executed.
- [ ] PR description documents scope, behavior, verification, and limitations.
