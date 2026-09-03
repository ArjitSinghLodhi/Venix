# 🦀 Venix ECS Engine

A deterministic, high-concurrency Entity Component System (ECS) written in Rust, engineered for zero-overhead structural memory layouts, compile-time concurrency verification, and vectorized parallel execution pipelines.

---

## 🔒 Production Readiness & API Blueprint

Venix v3.x.x is the complete, stable baseline for the ecs. The core traits and architecture are locked down. You can build on this version without worrying about breaking changes or API churn.

* **SemVer Compliance**  
  The public API surface is stable. No unexpected signature rewrites in future updates.
* **Vanilla Rust Data Types**  
  Zero mandatory derive macros, procedural code generation, or structural marker traits. Your domain components stay clean, vanilla Rust structs.
* **Zero Magic Type-System Hacks**  
  For single component operations, a `(Component,)` is needed rather than a plain `Component` to avoid forcing derive traits on every struct. However, enabling the derive feature flag allows you to cleanly turn any struct into an unpackable component bundle.

---

## 🚀 Performance & Invariant Guarantees

* **🛡️ 100% Miri-Validated Sandbox**  
  Built safely on raw pointer offsets and dense tabular memory operations, passing full Miri verification with zero undefined behavior.
* **📦 Contiguous Archetype Grid**  
  Entities with identical component layouts pack contiguously into unified columns, maximizing CPU L1/L2 cache locality and enabling pure sequential vector loops.
* **⚡ Fork-Join Parallel Iterator**  
  Integrates a high-performance Rayon worker pool to partition and stream archetype data chunks concurrently across available CPU cores.
* **⏳ Command Synchronization**  
  Structural modifications (spawning, insertions, deletions) buffer into a concurrent queue and flush at the end of every frame.
* **🌐 De-coupled Thread Spawning**  
  Supports extracting concurrent execution handlers (`app.get_par_commands()`) completely outside system loops, allowing long-running background threads to safely queue entity spawns asynchronously.
* **📡 Thread-Independent Event Broadcasting**  
  Allows external background workers or network threads to pull standalone event handles (`app.get_par_event_writer::<T>()`) to broadcast global notifications out-of-band cleanly.

---

## 🎨 Example code

```rust
use venix::prelude::*;

struct FrameCounter {
    current_frame: u32,
}

fn test_runner_once(app: &mut App) {
    app.build();
    app.run_startup();
    
    while app.get_resource::<FrameCounter>().current_frame != 10 {
        app.update();
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultSchedulesPlugin)
        .insert_resource(FrameCounter { current_frame: 0 })
        .add_systems(Update::id(), hello_world_system)
        .set_runner(test_runner_once)
        .run();
}

fn hello_world_system(mut frame: ResMut<FrameCounter>) {
    frame.current_frame += 1;
    println!("hello world");
    println!("Current frame: {}", frame.current_frame);
}
```

---

## ⚠️ Lifecycle Constraints

Venix tracks runtime data modifications through an explicit double-buffered structural tracking network. 

> [!IMPORTANT]
> **The 1-Frame Visibility Rule:** Any data adjustment or property update evaluated via the `Changed<T>` or `Added<T>` filters remains visible to matching queries for a window of **exactly 1 execution frame**.

```text
 [ Frame N ]         ➔            [ Frame N+1 ]            ➔      [ Frame N+2 ]
Mutation Mutated                  Double Buffers Swapped          Modification Cleared
Tracker Updates Token Hidden      Visible to Queries              Token Decays / Dropped
```

* **Frame N (Mutation Origin):** Values are changed. Internal trackers update and are hidden from active reads.
* **Frame N+1 (Reactive Window):** Structural buffers swap. Filtered queries intercept, read, and evaluate changes.
* **Frame N+2 (Buffer Decay):** Mutation tokens overwrite automatically. Visibility drops, and query matching states are back to normal.

*Note: All reactive logic mapping tracking events via filters must dispatch within this strict 1-frame boundary. Custom runners or delaying system ticks past this lifecycle window results in immediate mutation visibility decay.*

---

### The Entity Despawn Invariant

Venix enforces a strict handles invariant to maintain memory safety and structural integrity across parallel schedules.

> [!IMPORTANT]
> **The Rule:** All cloned handles referencing an entity must be completely dropped before that entity's queued despawn command is applied.

* **Deferred Execution:** Calling `commands.despawn(entity)` does not kill the entity or panic right away; it merely registers a deferred command to be processed later.
* **The Panic:** The engine panics during the command execution phase if any cloned handles for that target entity are still active when the queue flushes.
* **The Diagnostic:** The panic text prints a clean `HashSet` containing the exact `std::any::type_name` of every component within that entity's archetype, making it easy to identify the problematic entity type.
* **The Resolution:** For projects using the `DefaultSchedulesPlugin`, look into its documentation to understand how some schedules are deliberately structured to help you use `despawn_iter` and `will_despawn` to satisfy this requirement.

---

## 🛠️ Feature & Module Matrix

### Procedural Macro Derives (`feature = "derive"`)
Unlock zero-overhead data abstractions. Code generation pipelines maintain user encapsulation rules, safely respecting struct/field privacy constraints (`pub`, `pub(crate)`):
* `#[derive(ComponentBundle)]` – Collects individual types into uniform data groups.
* `#[derive(QueryData)]` – Maps fields directly to underlying structural archetype columns.
* `#[derive(QueryFilter)]` – Unifies condition filter into narrow single structs.
* `#[derive(SystemParam)]` – Groups system parameters into unified struct.

### Core Ecosystem Modules
* `venix::prelude` – Includes all normal everyday usage imports.
* `venix::extensions` – Includes lower level access to archetypes and data for anyone to build ontop of venix.

### Cargo Compilation Flags

Venix by default compiles strictly as a data container (`default = []`). Scale the engine's capabilities by opting into modular compilation blocks in your `Cargo.toml`:

* `derive` – Activates code-generation syntax macros (`ComponentBundle`, `QueryData`, etc.).
* `reactivity` – Activates double-buffered reactivity tracking (`Added`, `Changed`, `ChangedTracker`, etc.).
* `events` – Activates high-concurrency event broadcasting pipelines (`EventWriter`, `EventReader`, `ParallelEventWriter`, etc.).

---

## 📜 License

Venix is dual-licensed under either:

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
* MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
