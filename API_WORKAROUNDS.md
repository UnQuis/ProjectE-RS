# Horizon API workarounds

This file records deliberate compatibility workarounds. It is not a list of preferred APIs.
When the pinned Horizon API grows the missing trait-mod accessors, replace these implementations
in one migration and update this document.

Pinned Horizon revision: `fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca`.

## 1. Machine EMC: block-entity NBT instead of native energy access

**Current implementation**

- Collector and Relay store EMC in their own block entity's saved NBT compound, under `emc`.
- The Collector's current buffer is updated directly in `BlockEntityTickEvent.entity`.
- A one-time compatibility migration reads the legacy `projecte:buffer` attachment from PR #3 and moves it into the `emc` NBT field on the machine's first tick.
- Adjacent Relay block entities are modified through `Host::change_mod_block_entity`.
- Machine collection clears the NBT value, credits the player's ProjectE attachment through a fallible write, and attempts to restore the machine buffer if that write fails.
- The storage is server-authoritative and persists with the block entity.

**Why this is a workaround**

Horizon has the `block_entity_energy` capability hook (API 142), and its engine documents native energy operations. However, those operations are not exposed as callable methods on the pinned trait-mod `Host` surface / special Host bridge. The bridge does expose neighbouring mod block entities, which is enough for ProjectE-owned EMC transfer.

We therefore cannot safely use Horizon's energy handler as the source of truth from this trait mod. A ProjectE-owned NBT field keeps the gameplay functional without pretending that native energy access is available.

**Migration when Host access is added**

1. Preserve the one-time PR #3 attachment migration before replacing the NBT backend.
2. Register `ModEnergyBuffer` for each machine through `block_entity_energy`.
3. Replace reads/writes of `emc` with the official energy accessors.
4. Route collector-to-relay transfer through the official transfer helper.
5. Migrate old `emc` values to the native buffer exactly once; never initialize both stores independently.
6. Remove the NBT workaround and its migration code only after old worlds are covered.

## 2. Klein Star charge: ProjectE custom component instead of native item energy

**Current implementation**

- All six Klein Star tiers declare a ProjectE-owned `projecte:stored_emc` component.
- The component is read and updated through `Host::component`, `Host::set_component` and `Host::remove_component`; values are clamped to the tier's capacity.
- Collector slots 0 and 1 charge Klein Stars from its buffer. Relay slot 0 empties a star into its buffer; slot 1 charges a star. A full star moves to output slot 2.
- The tooltip displays stored EMC and capacity.

**Why this is a workaround**

Horizon has the `item_energy_buffer` capability hook (API 307), but the pinned trait-mod Host surface does not expose native item-energy read/insert/extract methods. The component Host bridge is available, so ProjectE stores charge in its own registered component.

**Migration when Host access is added**

1. Register `ItemEnergyBuffer` for each Klein Star tier.
2. Replace component reads/writes with native item-energy operations.
3. Preserve single-item stack semantics: Horizon's item-energy operation returns a modified one-item stack; split and replace stacks correctly.
4. Migrate existing `projecte:stored_emc` values into `horizon_energy` once.
5. Remove the custom component registration only after migration support is in place.

## 3. Neighbour transfer through block-entity mutation

`Host::mod_block_entity` returns a copy for reading. `Host::change_mod_block_entity` applies a mutation to the live block entity. ProjectE uses the latter for writes and only calls it from server-authoritative machine ticks or player interactions.

Do not replace this with a read-copy-write sequence: two machines could otherwise act on stale snapshots and duplicate or lose EMC.

## 4. Scope and known limitations

- Collector and Relay now have three tiers each. This remains a partial machine port, not the complete ProjectE device lineup.
- Collector tiers generate 4/12/40 EMC per second in full sunlight, with capacities 10,000/30,000/60,000 and fractional EMC accumulation. Relay tiers have capacities 100,000/1,000,000/10,000,000, transfer limits 64/192/640 EMC per tick, and transfer bonuses of 0.05/0.15/0.5 EMC per successful transfer tick.
- Collectors feed adjacent Relays. Relays feed adjacent Relays only when the target has a lower fill ratio, reducing back-and-forth transfers. Machine slots are implemented; Collector fuel upgrades and Condensers are not implemented yet.
- Klein Stars have persistent component storage and can be charged/discharged through Collector and Relay machine slots. Direct player-inventory transfer is not implemented yet.
- Attachment read/write errors in the existing player-data layer are a separate cleanup item; do not silently copy those patterns into the energy backend.
