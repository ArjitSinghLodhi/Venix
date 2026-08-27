use rusty_fork::rusty_fork_test;
use std::thread;
use venix::events::{EventReader, EventWriter};
use venix::prelude::*;

#[derive(Debug, Clone)]
struct ThreatEvent {
    value: f32,
}

struct FrameCounter {
    current_frame: u32,
}

fn increment_frame_system(mut counter: ResMut<FrameCounter>) {
    counter.current_frame += 1;
}

fn pre_emit_verify_system(counter: Res<FrameCounter>, reader: EventReader<ThreatEvent>) {
    let count = reader.read_scope(|iter| iter.count());
    assert!(
        reader.read_scope(|mut iter| iter.all(|event| event.value == 10.0)),
        "Value corrupted or not same"
    );
    if counter.current_frame == 1 {
        assert_eq!(
            count, 0,
            "❌ Soundness Failure: Pre-emit system found an event on Frame 1 before any emission occurred! Got: {}, Expected: 0",
            count
        );
    } else if counter.current_frame == 2 {
        assert_eq!(
            count, 1,
            "❌ Core Failure: Pre-emit system failed to pick up the historical inter-frame event on Frame 2! Got: {}, Expected: 1",
            count
        );
    } else if counter.current_frame == 3 {
        assert_eq!(
            count, 0,
            "❌ Stagnation Failure: Event leaked into Frame 3 for the Pre-emit system after it should have stagnated out of bounds! Got: {}, Expected: 0",
            count
        );
    }
}

fn emitter_system(counter: Res<FrameCounter>, mut writer: EventWriter<ThreatEvent>) {
    if counter.current_frame == 1 {
        writer.send(ThreatEvent { value: 10.0 });
    }
}

fn post_emit_verify_system(counter: Res<FrameCounter>, reader: EventReader<ThreatEvent>) {
    let count = reader.read_scope(|iter| iter.count());
    assert!(
        reader.read_scope(|mut iter| iter.all(|event| event.value == 10.0)),
        "Value corrupted or not same"
    );
    if counter.current_frame == 1 {
        assert_eq!(
            count, 1,
            "❌ Core Failure: Post-emit system failed to catch the immediate inline event alteration on Frame 1! Got: {}, Expected: 1",
            count
        );
    } else if counter.current_frame == 2 {
        assert_eq!(
            count, 0,
            "❌ Pipeline Leak: Post-emit system flagged an event on Frame 2 that it already consumed inline on Frame 1! Got: {}, Expected: 0",
            count
        );
    } else if counter.current_frame == 3 {
        assert_eq!(
            count, 0,
            "❌ Stagnation Failure: Event leaked into Frame 3 for the Post-emit system after it should have stagnated out of bounds! Got: {}, Expected: 0",
            count
        );
    }
}

fn test_runner_frames(app: &mut App) {
    app.build();
    app.run_startup();

    app.update();
    app.update();
    app.update();
}

rusty_fork_test! {
    #[test]
    fn test_events_multi_frame_lifecycle() {
        let mut app = App::new();
        app.add_plugins(DefaultSchedulesPlugin)
            .insert_resource(FrameCounter { current_frame: 0 })
            .init_event::<ThreatEvent>()
            .add_systems(
                Update::id(),
                (
                    increment_frame_system,
                    pre_emit_verify_system,
                    emitter_system,
                    post_emit_verify_system,
                ),
            );

        app.set_runner(test_runner_frames);
        app.run();
    }
}

fn parallel_execution_system(
    counter: Res<FrameCounter>,
    reader: EventReader<ThreatEvent>,
    mut writer: EventWriter<ThreatEvent>,
    par_writer: ParallelEventWriter<ThreatEvent>,
) {
    if counter.current_frame == 1 {
        writer.send(ThreatEvent { value: 100.0 });

        thread::scope(|s| {
            let reader_ref = &reader;
            let par_writer_ref = &par_writer;

            s.spawn(move || {
                let _ = reader_ref.read_scope(|iter| iter.count());
            });

            s.spawn(move || {
                par_writer_ref.scope(|mut scoped_writer| {
                    for _ in 0..5 {
                        scoped_writer.send(ThreatEvent { value: 10.0 });
                    }
                });
            });

            s.spawn(move || {
                par_writer_ref.scope(|mut scoped_writer| {
                    for _ in 0..5 {
                        scoped_writer.send(ThreatEvent { value: 10.0 });
                    }
                });
            });
        });
    }
}

fn parallel_verification_system(counter: Res<FrameCounter>, reader: EventReader<ThreatEvent>) {
    let count = reader.read_scope(|iter| iter.count());

    if counter.current_frame == 1 {
        assert_eq!(count, 11);

        let mut base_count = 0;
        let mut parallel_count = 0;

        reader.read_scope(|iter| {
            for event in iter {
                if event.value == 100.0 {
                    base_count += 1;
                } else if event.value == 10.0 {
                    parallel_count += 1;
                }
            }
        });

        assert_eq!(base_count, 1);
        assert_eq!(parallel_count, 10);
    } else if counter.current_frame == 2 {
        assert_eq!(count, 0);
    }
}

fn test_two_frames(app: &mut App) {
    app.build();
    app.run_startup();

    app.update();
    app.update();
}

#[test]
fn test_events_parallel_lifecycle() {
    let mut app = App::new();
    app.add_plugins(DefaultSchedulesPlugin)
        .insert_resource(FrameCounter { current_frame: 0 })
        .init_event::<ThreatEvent>()
        .add_systems(
            Update::id(),
            (
                increment_frame_system,
                parallel_execution_system,
                parallel_verification_system,
            ),
        );

    app.set_runner(test_two_frames);
    app.run();
}
