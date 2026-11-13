# Horizon API notes for ProjectE-RS

> **Pinned Horizon source:** `87247cac7cedfe0ddc455e1485ba1154b6089e5b`  
> **Pinned horizon-mixins tool:** `e59da9094f0c2fdda6c627fe9eff22dec6a906f0`  
> **Minecraft target:** `26.3`  
> **Native mod manifest:** `engine/horizon-mod.toml`

This document tracks the native-mod API ProjectE-RS targets. The pins in the manifest are authoritative:
do not use Horizon `main`, a floating Git dependency, or a locally installed checkout when changing code.

## 1. Build model

ProjectE-RS is a native mod package rooted at `engine/`:

- `horizon-mod.toml` declares the mod identity and exact `[native]` pins.
- `Cargo.toml` depends on the official `horizon-mod-api` crate at the same Horizon commit.
- `src/lib.rs` contains the single `#[mod_api::horizon_mod]` implementation.
- `data/` contains the mod's data-pack resources.

The launcher compiles the mod against the prebuilt SDK for the exact Horizon commit. It does not rebuild the game.
When native mixins are declared, the pinned `horizon-mixins` tool patches the prebuilt game functions. A mixin
target is tied to the exact Horizon source revision and must be re-audited whenever the pin changes.

CI follows the same boundary: it resolves the pinned API dependency and runs rustfmt, Clippy, cargo check and
tests for `engine/Cargo.toml`. It must never check out the Horizon game source or build the game as a CI
prerequisite for ordinary mod changes.

## 2. Extension model

Prefer the least invasive extension point:

1. **Hooks** for global game events and registration batches.
2. **Parts** for behavior attached to a particular mod block, block entity, item or menu. The new API separates
   global hooks from object-scoped behavior; a hook with a `scope` and `key` in the vocabulary is implemented
   by the corresponding Part.
3. **Injections** for callbacks at a function boundary when a normal hook does not cover the moment.
4. **Native mixins** only when private game state, a mid-function operation, or a replacement body is genuinely
   required.

The `#[mod_api::horizon_mod]` attribute generates the `hooks()` factory and subscribes the mod to the Hooks
methods implemented in the marked crate-root impl. Do not define a second factory manually. The mod type must
implement `Default`.

## 3. Energy capabilities

The pinned API exposes the native energy surface:

- `block_entity_energy` declares a mod block entity's saved energy buffer, with capacity, per-call insert and
  extract limits, and permitted faces.
- `ENERGY` is the block capability key; its handler is an `EnergyHandler`.
- `move_energy(host, from, to, max, tx)` transfers energy transactionally and rejects over-answering handlers.
- `ITEM_ENERGY` is the item capability key. Item-place exchanges split a changed single item from a larger
  stack, so Klein Star operations must preserve stack semantics.
- `furnace_burn` is a listener that reports consumed fuel; it does not itself alter burn duration. Changing
  alchemical fuel duration requires a verified native mixin target.

Machine NBT and `projecte:stored_emc` compatibility data must be migrated exactly once. Never initialize the
native buffer and the legacy store independently, and never delete legacy data before the migration path is
covered by tests.

## 4. Compatibility and safety

Native code runs with the player's operating-system permissions; it is not sandboxed. The launcher requires the
user to trust native code before building it. Do not introduce unnecessary dependencies or native mixins.

Horizon API contracts are experimental and revision-specific. Confirm hook signatures, Part methods, host
capabilities, and mixin selectors against the exact commit above before implementation. The API docs linked from
the pinned source are the reference, not remembered signatures from an older trait-mod API.
