# Venix ECS Engine

A deterministic, high-concurrency Entity Component System written in Rust. Venix provides zero-overhead structural memory layouts, compile-time concurrency verification, and vectorized parallel execution pipelines.

---

## Performance & Invariant Guarantees

* **Validated by Miri:** The internal engine utilizes raw pointer offsets and dense tabular memory operations. The entire codebase compiles with zero undefined behavior and strictly passes full miri verification (Note: currently ParallelEventWriter does not pass Miri's stacked borrow rules).
* **Dense Archetype Architecture:** Entities sharing identical component configurations are packed contiguously into unified archetype tables. This layout ensures sequential vector access, eliminates indirect pointer hopping, and maximizes CPU L1/L2 cache line utility.
* **Work-Stealing Concurrency:** Native integration with a high-performance Rayon worker pool enables parallel processing over archetype data batches without runtime dispatch overhead.
* **Static Access Routing:** Query parameter bounds constraints (With, Without, Changed) to resolve structural archetype filtering paths instantly before execution loops initiate.
* **Linear Synchronized Commits:** Mutative operations—including lifecycle spawning, structural insertions, and removals—are deferred into thread-local ParallelCommands buffers. Modifications are flushed linearly at system boundary synchronization checkpoints to maintain reference stability.
* **Events:** EventWriter and EventReader are both supported they use the same lifecycle logic as Changed detection.

---

## ⚠️ State Evolution Lifecycle Constraints

Venix tracks runtime data modifications through an explicit generational bitmask pipeline.

**Operational Rule:** Any structural property change or mutable access evaluated via the Changed filter remains valid for a window of exactly 2 execution frames.

* **Generation Frame N:** Data properties are altered. An internal tracker initializes the change token.
* **Generation Frame N+1:** The modification state is actively maintained. This allows downstream filtered queries to identify and react to the change event.
* **Generation Frame N+2:** The generation counter shifts. The modification token is overwritten or reset, and its visibility is dropped.

Note: All logic tracking changes via Changed filters must execute within this 2-frame boundary. If custom runner architectures delay system dispatch past this window, the mutation visibility is lost.

---

## Feature Overview

### Procedural Macro Derives
Enable the derive feature flag to unlock zero-cost data abstractions. The engine's code generation framework automatically handles visibility and structure:

* `#[derive(ComponentBundle)]` – Packs loose components into cohesive spawning layouts.
* `#[derive(QueryData)]` – Maps fields directly to underlying raw archetype columns.
* `#[derive(QueryFilter)]` – Combines multiple composition criteria into specialized filters.
* `#[derive(SystemParam)]` – Groups complex queries, thread-local command buffers, and shared resources into unified system signatures.

### Extensibility Core
* **prelude:** Re-exports standard primitives, queries, scheduling components, and primitives for everyday application usage.
* **extensions:** Exposes low-level archetypal columns, access trackers, and execution handles for anyone to add on top of venix.

---

## License

Venix is dual-licensed under either:

* Apache License, Version 2.0 (LICENSE-APACHE)
* MIT license (LICENSE-MIT)

at your option.
