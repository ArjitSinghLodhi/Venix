use std::thread;

use rusty_fork::rusty_fork_test;
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

rusty_fork_test! {
    #[test]
    fn test_commands_and_resources() {
        let mut app = App::new();
        app.add_plugins(DefaultSchedulesPlugin)
            .insert_resource(ScoreTracker { points: 0 })
            .add_systems(Startup::id(), setup_initial_entities)
            .add_systems(Startup::id(), test_par_commands)
            .add_systems(Update::id(), verify_resource_and_commands);

        app.set_runner(test_runner_once);
        app.run();
    }
}

fn spawn_filter_targets(mut commands: Commands) {
    commands.spawn((Position { x: 1.0, y: 1.0 }, TagA));
    commands.spawn((Position { x: 2.0, y: 2.0 }, TagA));
    commands.spawn((Position { x: 3.0, y: 3.0 }, TagB));
}

fn verify_logical_queries(
    or_query: Query<&Position, Or<With<TagA>, With<TagB>>>,
    not_query: Query<&Position, (With<TagA>, Not<With<TagB>>)>,
) {
    let mut or_count = 0;
    for chunk in or_query.iter() {
        or_count += chunk.len();
    }
    assert_eq!(or_count, 3);

    let mut not_count = 0;
    for chunk in not_query.iter() {
        for pos in chunk.iter() {
            if pos.x == 2.0 {
                not_count += 1;
            }
        }
    }
    assert_eq!(not_count, 1);
}

rusty_fork_test! {
    #[test]
    fn test_query_filter_logic() {
        let mut app = App::new();
        app.add_plugins(DefaultSchedulesPlugin)
            .add_systems(Startup::id(), spawn_filter_targets)
            .add_systems(Update::id(), verify_logical_queries);

        app.set_runner(test_runner_once);
        app.run();
    }
}

fn spawn_tracking_entity(mut commands: Commands) {
    commands.spawn((Position { x: 5.0, y: 5.0 }, Velocity { x: 1.0, y: 1.0 }));
}

fn modify_component_system(mut query: Query<&mut Position>) {
    for mut chunk in query.iter_mut() {
        for mut pos in chunk.iter_mut() {
            pos.x = 50.0;
        }
    }
}

fn verify_change_tracking(query: Query<&Position, Changed<Position>>) {
    let mut change_detected = false;
    for chunk in query.iter() {
        for pos in chunk.iter() {
            if pos.x == 50.0 {
                change_detected = true;
            }
        }
    }
    assert!(change_detected);
}

fn verify_change_tracking_data(query: Query<(&Position, ChangedTracker<Velocity>)>) {
    let mut change_detected_track = false;
    for chunk in query.iter() {
        for (pos, mark) in chunk.iter() {
            if pos.x == 50.0 && !mark.is_changed() {
                change_detected_track = true;
            }
        }
    }
    assert!(change_detected_track);
}

rusty_fork_test! {
    #[test]
    fn test_changed_generational_tracking() {
        let mut app = App::new();
        app.add_plugins(DefaultSchedulesPlugin)
            .add_systems(Startup::id(), spawn_tracking_entity)
            .add_systems(
                Update::id(),
                (modify_component_system, verify_change_tracking, verify_change_tracking_data),
            );

        app.set_runner(test_runner_once);
        app.run();
    }
}

pub struct FrameCounter {
    pub current_frame: u32,
}

fn setup_multi_frame_entity(mut commands: Commands) {
    commands.spawn((Position { x: 10.0, y: 10.0 },));
}

fn increment_frame_system(mut counter: ResMut<FrameCounter>, mut query: Query<&mut Position>) {
    counter.current_frame += 1;
    if counter.current_frame == 2 {
        for mut chunk in query.iter_mut() {
            for mut pos in chunk.iter_mut() {
                pos.x = 20.0;
            }
        }
    }
}

fn verify_multi_frame_tracking(
    counter: Res<FrameCounter>,
    changed_query: Query<&Position, Changed<Position>>,
) {
    if counter.current_frame == 1 {
        let mut changes = 0;
        for chunk in changed_query.iter() {
            changes += chunk.len();
        }
        assert_eq!(changes, 0);
    }

    if counter.current_frame == 2 {
        let mut change_detected = false;
        for chunk in changed_query.iter() {
            for pos in chunk.iter() {
                if pos.x == 20.0 {
                    change_detected = true;
                }
            }
        }
        assert!(
            change_detected,
            "Frame 2 failed to detect component alteration!"
        );
    }

    if counter.current_frame == 3 {
        let mut changes_on_frame_three = 0;
        for chunk in changed_query.iter() {
            changes_on_frame_three += chunk.len();
        }
        assert_eq!(
            changes_on_frame_three, 0,
            "Stale change marker leaked because the system did not clear its generation block!"
        );
    }
}

fn test_runner_three_frames(app: &mut App) {
    app.build();
    app.run_startup();

    app.update();
    app.update();
    app.update();
}

rusty_fork_test! {
    #[test]
    fn test_multi_frame_lifecycle_and_stagnation() {
        let mut app = App::new();
        app.add_plugins(DefaultSchedulesPlugin)
            .insert_resource(FrameCounter { current_frame: 0 })
            .add_systems(Startup::id(), setup_multi_frame_entity)
            .add_systems(
                Update::id(),
                (increment_frame_system, verify_multi_frame_tracking),
            );

        app.set_runner(test_runner_three_frames);
        app.run();
    }
}
