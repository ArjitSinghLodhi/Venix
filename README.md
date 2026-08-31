# 🦀 Venix ECS Engine

A deterministic, high-concurrency Entity Component System (ECS) written in Rust, engineered for zero-overhead structural memory layouts, compile-time concurrency verification, and vectorized parallel execution pipelines.

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

## 🛠️ Feature & Module Matrix

### Procedural Macro Derives (`feature = "derive"`)
Unlock zero-overhead data abstractions. Code generation pipelines maintain user encapsulation rules, safely respecting struct/field privacy constraints (`pub`, `pub(crate)`):
* `#[derive(ComponentBundle)]` – Collects individual types into uniform data groups.
* `#[derive(QueryData)]` – Maps fields directly to underlying structural archetype columns.
* `#[derive(QueryFilter)]` – Unifies condition filter into narrow single structs.
* `#[derive(SystemParam)]` – Groups system parameters into unified struct.

### Core Ecosystem Modules
* `venix::prelude` – Includes all normal every day usage imports.
* `venix::extensions` – Includes lower level access to archetypes and data for anyone to build ontop of venix.

### Cargo Compilation Flags
* `derive` – Activates code-generation syntax macros (`ComponentBundle`, `QueryData`, etc.).
* `reactivity` – Activates reactivity mechanics like `Added` and `Changed`.

---

## 📜 License

Venix is dual-licensed under either:

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
* MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
