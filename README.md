# Venix ECS Engine

A deterministic, high-concurrency Entity Component System written in pure, checked Rust. This framework provides zero-overhead memory layouts, structural safety guarantees under strict compile-time/startup runtime verification, and isolated execution pipelines.

## Architectural Design

* **Guaranteed Memory Safety:** Operates with absolute zero undefined behaviour. Component Lifetimes, simultaneous aliasing rules, and cross-thread access patterns are validated by the compiler.
* **Dense Archetype Layouts:** Organizes entities sharing identical component patterns into unified memory blocks. This approach speeds up linear iterations, enhances CPU data cache line usage, and supports localized memory operations.
* **Native Thread Scaling:** Integrates with the Rayon work-stealing thread pool to distribute entity batches dynamically across logical processor cores during complex simulation loops.
* **Type-Filter Routing:** Uses standard type constraints like With, Without, and state tracking filters like Changed to isolate queries instantly without dynamic inspection costs.
* **Deferred Structural Commits:** Schedules operational shifts—including object lifecycle mutations or component insertions—into isolated Command queues. Modifications execute linearly at system boundary sync points to keep active iteration views stable.
* **Global Configuration Management:** Provides safe execution methods to update isolated shared parameters using centralized resource references alongside standard component arrays.

---

## ⚠️ State Evolution Lifecycle Limitation

The runtime uses an explicit generational ticking approach to record state changes within the Changed query filter pipeline.

**Operational Rule:** Any structural property change can only be safely evaluated for a window of **exactly 2 frames**.

* **Generation Frame N:** Properties are altered; a local flag tracks the initialization shift.
* **Generation Frame N+1:** The bitmask is retained, allowing connected downstream evaluation tasks to run.
* **Generation Frame N+2:** The state tracker is reset or overwritten.

Systems filtering tasks via Changed must complete their logic updates within this restricted 2-frame window. If execution is delayed past this interval by custom schedule runner or something else, the target data flags drop and the modification visibility is lost and has a possibility of breaking things.

---

## Extras
While Prelude module gives you everyday things you need imported.

extensions module gives you extra things you can use to make your own custom params possibly too.

and a lot of public traits which you can do many things with