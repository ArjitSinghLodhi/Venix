use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use venix::prelude::*;

pub struct Transform {
    pub matrix: [f32; 16],
}
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}
pub struct Rotation {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}
pub struct Velocity {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub struct StructuralTagA;
pub struct StructuralTagB;
pub struct StructuralTagC;

#[derive(Default)]
pub struct BenchState {
    pub initialized: bool,
}

criterion_main!(benches);

fn setup_fragmented_world(mut commands: Commands, mut state: ResMut<BenchState>) {
    println!("[ECS Bench Setup] Spawning 10,000 highly fragmented entities...");

    for i in 0..10_000 {
        let transform = Transform { matrix: [0.0; 16] };
        let position = Position {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        let rotation = Rotation {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        let velocity = Velocity {
            x: 1.0,
            y: 1.0,
            z: 1.0,
        };

        match i % 4 {
            0 => {
                commands.spawn((transform, position, rotation, velocity, StructuralTagA));
            }
            1 => {
                commands.spawn((transform, position, rotation, velocity, StructuralTagB));
            }
            2 => {
                commands.spawn((transform, position, rotation, velocity, StructuralTagC));
            }
            _ => {
                commands.spawn((transform, position, rotation, velocity));
            }
        }
    }

    state.initialized = true;
    println!("[ECS Bench Setup] Fragmentation complete. 4 distinct archetypes generated.");
}

fn run_fragmented_iter_bench(mut query: Query<(&mut Position, &Velocity)>, state: Res<BenchState>) {
    if !state.initialized {
        return;
    }

    let mut c = Criterion::default().configure_from_args();

    c.bench_function("ecs_suite_fragmented_iter", |b| {
        b.iter(|| {
            for mut view in query.iter_mut() {
                #[allow(unused_mut)]
                for (mut pos, vel) in view.iter_mut() {
                    pos.x += vel.x;
                    pos.y += vel.y;
                    pos.z += vel.z;
                    black_box(pos);
                }
            }
        });
    });
}

fn run_bench_timeline(app: &mut App) {
    app.build();
    app.run_startup();
    app.update();
}

fn bench_entry_point(_c: &mut Criterion) {
    App::new()
        .add_plugins(DefaultSchedulesPlugin)
        .insert_resource(BenchState::default())
        .add_systems(Startup::id(), setup_fragmented_world)
        .add_systems(Update::id(), run_fragmented_iter_bench)
        .set_runner(run_bench_timeline)
        .run();
}

criterion_group!(benches, bench_entry_point);
