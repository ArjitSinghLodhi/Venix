use std::thread;
use venix::prelude::*;
pub struct Position {
    pub x: f32,
    pub y: f32,
}

pub struct Velocity {
    pub x: f32,
    pub y: f32,
}

pub struct TagA;
pub struct TagB;

pub struct ScoreTracker {
    pub points: i32,
}

fn test_runner_once(app: &mut App) {
    app.build();
    app.run_startup();
    app.update();
}

fn setup_initial_entities(
    mut commands: Commands,
    mut tracker: ResMut<ScoreTracker>,
    par_commands: ParallelCommands,
) {
    tracker.points = 100;

    commands.spawn((
        Position { x: 10.0, y: 20.0 },
        Velocity { x: 1.0, y: 1.0 },
        TagA,
    ));
    par_commands.scope(|mut cmd| {
        cmd.spawn((Position { x: 1.0, y: 1.0 }, TagB));
    });
    thread::scope(|s| {
        s.spawn(|| {
            par_commands.scope(|mut cmd| {
                cmd.spawn((Position { x: 1.0, y: 1.0 }, TagB));
            });
        });
        s.spawn(|| {
            par_commands.scope(|mut cmd| {
                cmd.spawn((Position { x: 1.0, y: 1.0 }, TagB));
            });
        });
    });
}

fn verify_resource_and_commands(
    tracker: Res<ScoreTracker>,
    query: Query<&Position, With<TagA>>,
    query_b: Query<&Position, With<TagB>>,
) {
    assert_eq!(tracker.points, 100);

    let mut found = false;
    for chunk in query.iter() {
        for pos in chunk.iter() {
            if pos.x == 10.0 && pos.y == 20.0 {
                found = true;
            }
        }
    }
    let mut found_b = 0;
    for chunk in query_b.iter() {
        for pos in chunk.iter() {
            if pos.x == 1.0 && pos.y == 1.0 {
                found_b += 1;
            }
        }
    }
    assert!(found);
    assert_eq!(found_b, 4);
}

fn test_par_commands(mut commands: Commands, par_commands: ParallelCommands) {
    commands.spawn((
        Position { x: 0.0, y: 0.0 },
        Velocity { x: 0.0, y: 0.0 },
        TagA,
    ));
    std::thread::scope(|s| {
        s.spawn(|| {
            par_commands.scope(|mut cmd| {
                cmd.spawn((Position { x: -50.0, y: 10.0 }, TagB));
                cmd.spawn((Position { x: -60.0, y: 15.0 }, TagB));
            });
        });
        s.spawn(|| {
            par_commands.scope(|mut cmd| {
                cmd.spawn((Position { x: 50.0, y: -10.0 }, TagB));
                cmd.spawn((Position { x: 60.0, y: -15.0 }, TagB));
            });
        });
    });
    par_commands.scope(|mut cmd1| {
        cmd1.spawn((Position { x: 1.0, y: 1.0 }, TagB));
        par_commands.scope(|mut cmd2| {
            cmd2.spawn((Position { x: 2.0, y: 2.0 }, TagB));
        });
    });
}

#[test]
fn test_commands_and_resources_single() {
    let mut app = App::new();
    app.add_plugins(DefaultSchedulesPlugin)
        .insert_resource(ScoreTracker { points: 0 })
        .add_systems(Startup::id(), setup_initial_entities)
        .add_systems(Startup::id(), test_par_commands)
        .add_systems(Update::id(), verify_resource_and_commands);

    app.set_runner(test_runner_once);
    app.run();
}
