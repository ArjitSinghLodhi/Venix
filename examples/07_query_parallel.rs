#[cfg(feature = "reactivity")]
compile_error!("🛑 This example cannot be compiled when 'reactivity' feature is on due to type changes introduced by the feature.");

use venix::prelude::*;
use rayon::iter::ParallelIterator;

#[allow(dead_code)]
struct WorkerId(u32);

#[allow(dead_code)]
struct Position { x: f32, y: f32 }
struct Velocity { x: f32, y: f32 }

fn main() {
    println!("=== TESTING VENIX: PARALLEL ARCHETYPE QUERIES ===\n");

    let mut app = App::new();
    app.add_plugins(DefaultSchedulesPlugin)
        .add_systems(Startup::id(), spawn_parallel_batch_system)
        .add_systems(
            Update::id(),
            (
                parallel_read_system,
                parallel_write_system,
                parallel_chunk_read_system,
                parallel_chunk_write_system,
            ),
        );

    app.set_runner(run_parallel_test_loop);
    app.run();

    println!("\n=== PARALLEL QUERY VERIFICATION COMPLETED ===");
}

fn spawn_parallel_batch_system(mut commands: Commands) {
    println!("Spawning dense data pools via fast batch allocation...");
    let batch = (0..20000).map(|i| {
        (
            Position { x: i as f32, y: 0.0 },
            Velocity { x: 1.0, y: 1.0 },
            WorkerId(0),
        )
    });
    commands.spawn_batch(batch);
}

fn parallel_read_system(query: Query<(&Position, &WorkerId)>) {
    query.par_iter().for_each(|view| {
        let chunk_count = view.len();
        if chunk_count > 0 {
            println!("[par_iter] Thread scanning view chunk of size: {}", chunk_count);
        }
    });
}

fn parallel_write_system(mut query: Query<(&mut Position, &Velocity)>) {
    let start = std::time::Instant::now();

    query.par_iter_mut().for_each(|mut view| {
        for (pos, vel) in view.iter_mut() {
            pos.x += vel.x;
            pos.y += vel.y;
        }
    });

    println!("[par_iter_mut] Completed view execution pass in {:?}", start.elapsed());
}

fn parallel_chunk_read_system(query: Query<(&Position, &Velocity)>) {
    for view in query.iter() {
        view.par_chunks(5000).for_each(|sub_chunk| {
            let sub_count = sub_chunk.len();
            println!("[par_chunks] Read worker swept partitioned chunk slice containing: {} rows", sub_count);
        });
    }
}

fn parallel_chunk_write_system(mut query: Query<(&mut Position, &Velocity)>) {
    let start = std::time::Instant::now();

    for mut view in query.iter_mut() {
        view.par_chunks_mut(5000).for_each(|mut sub_chunk| {
            let mut modified = 0;
            for (pos, vel) in sub_chunk.iter_mut() {
                pos.x += vel.x;
                pos.y += vel.y;
                modified += 1;
            }
            println!("[par_chunks_mut] Write worker mutably stepped row slice containing: {} rows", modified);
        });
    }

    println!("[par_chunks_mut] Multi-threaded sub-chunk splits executed in {:?}", start.elapsed());
}

fn run_parallel_test_loop(app: &mut App) {
    app.build();
    app.run_startup();

    app.update();
    app.update();
}
