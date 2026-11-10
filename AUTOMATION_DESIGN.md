# EMC automation architecture

Status: design proposal; not yet implemented.
Base: current `assistant/energy-system-port` implementation.
Target: Minecraft 26.3 on the pinned Horizon revision in `horizon-mixin.toml`.

This document defines the next automation layer without changing the current Collector, Relay, or Klein Star implementation. It is intentionally separate from `PROGRESSION.md`, which owns the complete player-facing progression roadmap.

## 1. Gameplay contract

The automation loop is:

```text
world light
   |
Energy Collector(s)
   |
Anti-Matter Relay network
   |
Energy Condenser
   |                 EMC-valued input items
target item          |
   |                 v
output inventory <- EMC buffer
```

A Collector is the source of passive EMC. A Relay is a buffer/router and applies its tier bonus when it successfully forwards Collector energy. A Condenser consumes EMC and/or EMC-valued input items to manufacture a selected target item.

A Collector may feed a Condenser directly. A Relay may feed a Condenser. A Condenser must never be treated as an EMC generator. It is an energy sink and item transmuter.

A Power Flower is an arrangement, not a separate block or special network object: multiple Collectors feed a Relay, which forwards their combined EMC to a Condenser. The implementation must support arbitrary valid adjacency arrangements rather than hard-coding a specific flower shape.

## 2. Condenser behavior

### 2.1 Target slot

- One dedicated target slot defines the item to manufacture.
- The target is a template and is never consumed by normal operation.
- A target is valid only if its resolved EMC value is strictly greater than zero.
- Changing or removing the target must not destroy the old target or any output.
- Target item identity includes all item components that affect item identity. Do not silently discard required components when creating output.
- If the target is invalid, preserve all input items and pause production with an explanatory UI state.

Knowledge learning is not a prerequisite for Condenser production. The Condenser is a machine that copies a valid EMC-valued item; the Transmutation Table's learned-item rule remains a separate player-facing restriction.

### 2.2 Input conversion

- Any inserted input stack with positive EMC may be consumed into the Condenser's internal EMC buffer.
- Inputs with zero or unknown EMC must remain untouched.
- Consume only the quantity that fits in the buffer and can be represented safely.
- Calculate `unit_emc * count` with checked arithmetic.
- Do not consume an item before its EMC credit is guaranteed.
- A failed block-entity write must not result in item loss.
- The target slot is excluded from input conversion.
- Inputs may arrive from player interaction or container automation if the pinned Horizon container API supports it.

### 2.3 Output production

- Output item cost is the target's EMC value.
- Produce an item only when the buffer contains at least that cost and output inventory has room.
- Debit EMC only for items actually inserted into output.
- If output is blocked or full, retain EMC without loss.
- Never overwrite an existing output stack or silently void overflow.
- Output stacks must respect the item's actual maximum stack size, not a hard-coded 64.
- The output slot/inventory must not be accepted as EMC input.

### 2.4 MK1 vs MK2

The two tiers share the same accounting and validation logic. They differ in throughput and inventory handling.

| Property | Condenser MK1 | Condenser MK2 |
|---|---|---|
| Input processing | One item per processing step | Batch processing, up to one stack per step |
| Output processing | One target item per processing step | Produce as many as the buffer and output capacity permit |
| Inventory | Shared input/output inventory plus target | Separate input and output inventories plus target |
| EMC source | Item inputs and adjacent Collector/Relay | Same |
| EMC sink | Target item production | Target item production |

The exact tick budget and buffer capacities are balance constants to be selected after comparing the existing Collector/Relay throughput. Do not guess them inside hook code. Put them in a single machine specification table with tests.

Batch processing must be bounded. Never iterate an unbounded inventory or allow a single tick to process an arbitrary number of stacks.

## 3. Shared EMC accounting

Introduce pure planning functions before Host integration. Suggested conceptual types:

- `CondenserTier`
- `CondenserSpec`
- `CondenserState`
- `InputConversionPlan`
- `OutputProductionPlan`
- `CondenserError`

Names may change to match repository conventions.

Required invariants:

1. Buffer is always in `0..=capacity`.
2. Every consumed input item is credited exactly once.
3. Every produced output item is charged exactly once.
4. A failed inventory mutation does not change the buffer.
5. A failed buffer mutation does not consume an input or grant output.
6. Partial stack acceptance is settled against the accepted count only.
7. All count and EMC arithmetic is checked.
8. Unknown/zero-value items are not silently assigned EMC.
9. Target item is never consumed as input.
10. The same EMC source of truth is used by the Transmutation Table, Relay item intake, and Condenser input conversion.

Avoid separate ad-hoc EMC formulas in machine tick hooks. Reuse `emc::value` and shared checked-cost helpers.

## 4. Network routing and Power Flower behavior

### 4.1 Recipient model

The current `machines::Machine` model only recognizes Collectors and Relays. Extend the routing model to represent Condensers as terminal recipients without pretending that a Condenser is another Relay.

Routing rules:

- Collector -> Relay: allowed.
- Collector -> Condenser: allowed.
- Relay -> Relay: allowed only when the destination has lower fill ratio, preserving the current anti-oscillation rule.
- Relay -> Condenser: allowed.
- Condenser -> any machine: forbidden.
- Condenser receives EMC up to its buffer capacity.
- Each source's per-tick transfer limit remains authoritative.
- Destination room is checked at mutation time, not only against a stale snapshot.

A source must debit only the amount confirmed as written to the live destination block entity. Use the existing live mutation bridge rather than read-copy-write.

### 4.2 Relay bonus

Keep Relay bonus accounting separate from transferred EMC.

- Bonus is awarded only when a Relay successfully forwards a positive amount of EMC originating from its buffer.
- A failed/blocked transfer awards no bonus.
- Fractional remainder persists across ticks.
- No bonus is awarded for merely receiving EMC or for charging a Klein Star.
- Tests must prove that routing to a Condenser follows the same bonus rules as routing to another Relay.

### 4.3 Avoiding loops

The network must not oscillate or duplicate energy.

- Condensers are terminal sinks.
- Relay-to-Relay routing continues to use a strictly decreasing fill-ratio condition.
- A source never credits a target based on a stale snapshot alone.
- If the live destination has less room than planned, credit only the actual amount accepted and debit exactly that amount from the source.
- If a source is ticked after a destination, energy may move on the next tick; this is acceptable and deterministic.

No global graph cache is required for the first implementation. Face-adjacent routing is sufficient and avoids lifecycle/invalidation complexity.

## 5. Block entity state and inventory

The current Collector/Relay use the `projecte:machine` block entity and three slots. Do not reuse the same block-entity id with incompatible inventory semantics unless the NBT schema remains backward-compatible.

Suggested persisted fields:

- `emc`: integer buffer.
- `target`: optional target item stack, including components.
- `input`: inventory payload for MK1/MK2.
- `output`: inventory payload for MK1/MK2 if separated.
- Optional processing remainder only if the implementation uses fractional throughput.

Use a versioned schema if a machine migration changes slot meanings. Never reinterpret old Collector/Relay slots as Condenser slots.

The current furnace-style container is only a temporary UI. Before adding more slots, verify the exact pinned Horizon `ModContainer`, block-entity inventory, menu synchronization, and slot transfer APIs. Do not assume a furnace menu can display a 91-slot inventory or custom target controls.

## 6. API and compatibility constraints

At the pinned Horizon revision, the current project uses:

- `Host::mod_block_entity` for neighbor snapshots.
- `Host::change_mod_block_entity` for live block-entity mutation.
- `Host::light`, `Host::day_time`, and `Host::weather` for Collector generation.
- ProjectE-owned NBT for machine EMC because native energy read/write helpers are not exposed through the pinned trait-mod Host surface.
- ProjectE-owned item component for Klein Star charge because native item-energy accessors are not exposed through the pinned trait-mod Host surface.

Do not assume that container automation, sided slot transfer, item maximum stack size, or block-entity inventory mutation APIs exist. Inspect the pinned Horizon source and generated Host bridge before implementation.

Any additional API workaround must be documented in `API_WORKAROUNDS.md`, with:
- missing API and evidence at the pinned revision;
- temporary implementation;
- persistence and multiplayer implications;
- migration plan once the API is exposed.

Do not implement a manual Data Map parser as a shortcut for EMC resolution.

## 7. UI requirements

Minimum functional UI:

- Visible target slot.
- Visible EMC buffer/progress.
- Distinguishable input and output areas.
- Clear indication when target has no EMC value.
- Clear indication when output is blocked.
- No client-side authority over EMC or item creation.

MK1 may initially use a smaller custom menu if the full upstream-style inventory is not practical. MK2 must keep input and output separated.

A generic furnace menu must not be presented as a finished Condenser interface if its slot layout does not match the actual machine semantics.

## 8. Collector fuel upgrades

Collector fuel upgrades are related to the automation loop but are a distinct feature from Condenser processing.

Treat them as a separate follow-up implementation:

- Define explicit fuel tiers and EMC-equivalent upgrade effects.
- Validate fuel item identity and count.
- Consume fuel only after its effect is committed.
- Keep fuel processing bounded per tick.
- Persist fractional progress/remainders.
- Ensure the fuel upgrade cannot generate EMC faster than its documented balance permits.
- Test night, weather, artificial light, and sky exposure independently from fuel effects.

Do not overload the Condenser input inventory or its target slot with Collector fuel behavior.

## 9. Test plan

Pure tests should cover:

- Valid and invalid target selection.
- Input items with zero, unknown, and positive EMC.
- Buffer capacity and partial input acceptance.
- Multiplication and addition overflow.
- MK1 one-item processing limits.
- MK2 stack batch limits.
- Output stack limits and blocked output.
- Debit only for successfully produced items.
- No consumption of target template.
- Direct Collector -> Condenser transfer.
- Relay -> Condenser transfer.
- Relay bonus on successful and unsuccessful forwarding.
- Relay routing cannot oscillate.
- Condenser cannot forward EMC.
- State encode/decode and schema migration.
- Old Collector/Relay state remains readable.

Host integration tests, if supported by Horizon, must additionally verify that a failed live block-entity mutation cannot duplicate or destroy EMC.

## 10. Implementation order

1. Verify pinned container/inventory APIs and document the exact usable methods.
2. Add pure Condenser accounting/planning and tests.
3. Add Condenser block/entity registration and versioned state.
4. Add MK1 menu and server-authoritative processing.
5. Extend Collector/Relay routing to the Condenser sink.
6. Add MK2 throughput and separate input/output.
7. Add Collector fuel upgrades as a separate change.
8. Add Power Flower recipes/advancements and end-to-end gameplay validation.

Do not implement all eight steps in one PR. Prefer a sequence of focused PRs, each based on the previous merged/accepted implementation.

## Public gameplay references

These references were used only to understand public, player-observable mechanics; no implementation source was copied:

- [Feed The Beast Wiki — Energy Condenser](https://ftb.fandom.com/wiki/Energy_Condenser_%28ProjectE%29)
- [Feed The Beast Wiki — Getting Started with ProjectE](https://ftb.fandom.com/wiki/Getting_Started_%28ProjectE%29)
- [Minecraft Guides — ProjectE overview](https://www.minecraft-guides.com/mod/projecte/)
- [Questing Wiki — ProjectE Power Flower](https://questing.wiki/ftb-presents-architects-exodus/guides/projecte-power-flower/)

Recipes, exact buffer sizes, and some tier behavior can vary by ProjectE version or modpack. Treat them as gameplay references, not as immutable implementation requirements.
