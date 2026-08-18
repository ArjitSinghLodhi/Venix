use rayon::iter::ParallelIterator;
use venix::prelude::*;
pub struct Database {
    data: Vec<i32>,
}
#[derive(Debug)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}
pub struct Velocity {
    pub x: f32,
    pub y: f32,
}
#[derive(Debug, Clone)]
pub struct Player;
pub struct Bullet;
pub struct Env;

fn movement_system(
    mut database: ResMut<Database>,
    mut query: Query<(Entity, &mut Position, &Velocity), (Without<Bullet>, With<Player>)>,
    mut bullets_query: Query<
        (Entity, &mut Velocity, &mut Position),
        (Without<Player>, With<Bullet>),
    >,
    mut env_query: Query<(Entity, &mut Position), (With<Env>, Without<Bullet>, Without<Player>)>,
    mut commands: Commands,
) {
    let mut entity_env = None;
    env_query.iter().for_each(|archetype_view| {
        for (entity, _) in archetype_view.iter() {
            entity_env = Some(entity.clone());
        }
    });
    for mut archetype_view in query.iter_mut() {
        archetype_view
            .iter_mut()
            .for_each(|(entity, mut pos, vel)| {
                pos.x += vel.x;
                println!("player_pos: {:?}", pos);
                if let Some(env_entity) = &entity_env {
                    let (_, mut env_pos) = env_query.get_mut(&env_entity).unwrap();
                    env_pos.x += 10.0;
                    println!("env_pos: {:?}", env_pos);
                }
                for _ in 0..1 {
                    commands.spawn((
                        Velocity { x: 1.0, y: 1.0 },
                        Bullet,
                        Position { x: 1.0, y: 1.0 },
                    ))
                }
                commands.despawn(entity.clone());
                println!("spawn");
            });
    }
    bullets_query.iter_mut().for_each(|mut archetype_view| {
        archetype_view
            .par_iter_mut()
            .for_each(|(_, mut vel, mut pos)| {
                vel.x += 1.0;
                pos.x += vel.x;
                if let Some(env_entity) = &entity_env {
                    let (_, env_pos) = env_query.get(&env_entity).unwrap();
                    println!("env_pos: {:?}", env_pos);
                }
                println!("thread_id: {:?}", rayon::current_thread_index());
            });
        archetype_view.iter_mut().for_each(|(bullet_entity, _, _)| {
            commands.add_components(bullet_entity.clone(), (Player,));
            commands.remove_components::<(Bullet,)>(bullet_entity.clone());
        });
    });
    database.data.push(1);
    println!("database: {:?}", database.data);
}

fn render_system(query: Query<&Position, (Changed<Position>, With<Velocity>, Changed<Velocity>)>) {
    println!("--- Frame Render Output ---");
    for chunk in query.iter() {
        for pos in chunk.iter() {
            println!("Rendered item location coordinates: {:?}", pos);
        }
    }
}

pub fn spawn_entities(mut commands: Commands) {
    commands.spawn((
        Position { x: 0.0, y: 0.0 },
        Velocity { x: 5.0, y: 10.0 },
        Player,
    ));
    for _ in 0..2 {
        commands.spawn((Position { x: 100.0, y: 100.0 }, Env));
    }
    for _ in 0..5 {
        commands.spawn((
            Velocity { x: 100.0, y: 100.0 },
            Bullet,
            Position { x: 1.0, y: 1.0 },
        ));
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultSchedulesPlugin)
        .insert_resource(Database {
            data: vec![1, 2, 3, 4, 5],
        })
        .add_systems(Startup::id(), spawn_entities)
        .add_systems(Update::id(), (movement_system, render_system))
        .run();
}
