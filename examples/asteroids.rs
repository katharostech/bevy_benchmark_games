use std::{f32::consts::PI, time::Instant};

use bevy::{
    app::AppExit, core::CorePlugin, prelude::*, type_registry::TypeRegistryPlugin,
    winit::WinitConfig,
};
use bevy_benchmark_games::{metrics::IterationMetrics, metrics::Metrics, random::FakeRand};

use rand::prelude::*;

struct Vel {
    x: f32,
    y: f32,
}

struct Asteroid;
struct Ship;
#[derive(Default)]
struct Bullet {
    alive_frames: u32,
}
#[derive(Default)]
struct BulletMaterial(Option<Handle<ColorMaterial>>);

#[cfg(headless)]
const RUN_FOR_FRAMES: usize = 2_000;
#[cfg(not(headless))]
const RUN_FOR_FRAMES: usize = 500;

#[cfg(headless)]
const ITERATIONS: usize = 50;
#[cfg(not(headless))]
const ITERATIONS: usize = 2;

fn spawn_ship(
    commands: &mut Commands,
    #[cfg(not(headless))] materials: &mut ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(SpriteComponents {
        #[cfg(not(headless))]
        material: materials.add(ColorMaterial::color(Color::rgb(0., 0., 1.))),
        transform: Transform::from_translation(Vec3::new(0., 0., 0.))
            .with_rotation(Quat::from_rotation_z(PI)),
        sprite: Sprite::new(Vec2::new(40., 20.)),
        ..Default::default()
    });
    commands.with(Ship);
}

fn setup(
    mut commands: Commands,
    #[cfg(not(headless))] mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let mut rng = FakeRand::new();
    commands.spawn(Camera2dComponents::default());

    // Spawn ship
    spawn_ship(
        &mut commands,
        #[cfg(not(headless))]
        &mut materials,
    );

    for _ in 0..100 {
        commands.spawn(SpriteComponents {
            #[cfg(not(headless))]
            material: materials.add(ColorMaterial::color(Color::rgb(
                rng.gen_range(0., 1.),
                rng.gen_range(0., 1.),
                rng.gen_range(0., 1.),
            ))),
            transform: Transform::from_translation(Vec3::new(
                rng.gen_range(-400., 400.),
                rng.gen_range(-400., 400.),
                0.,
            )),
            sprite: Sprite::new(Vec2::new(rng.gen_range(10., 50.), rng.gen_range(10., 50.))),
            ..Default::default()
        });
        commands.with(Vel {
            x: rng.gen_range(-2., 2.),
            y: rng.gen_range(-2., 2.),
        });
        commands.with(Asteroid);
    }
}

fn move_system(mut query: Query<(&mut Transform, &Vel)>) {
    for (mut trans, vel) in &mut query.iter() {
        trans.translate(Vec3::new(vel.x, vel.y, 0.))
    }
}

fn boundary_mirror(mut query: Query<With<Asteroid, &mut Transform>>) {
    for mut trans in &mut query.iter() {
        let mut pos = trans.translation();
        if pos.x() < -400. {
            pos.set_x(400.);
        } else if pos.x() > 400. {
            pos.set_x(-400.);
        }
        if pos.y() < -400. {
            pos.set_y(400.);
        } else if pos.y() > 400. {
            pos.set_y(-400.);
        }

        trans.set_translation(pos);
    }
}

#[derive(Default)]
struct MoveShipState {
    rng: FakeRand,
    frame_counter: u64,
}

fn move_ship(
    mut commands: Commands,
    mut state: Local<MoveShipState>,
    mut query: Query<With<Ship, &mut Transform>>,
) {
    state.frame_counter += 1;

    let frame_counter = state.frame_counter;
    let rng = &mut state.rng;

    for mut trans in &mut query.iter() {
        // rotate a random amount
        trans.rotate(Quat::from_rotation_z(rng.gen_range(-PI / 60., PI / 60.)));
        // move a random amount
        trans.translate(Vec3::new(
            rng.gen_range(-3., 3.),
            rng.gen_range(-3., 3.),
            0.,
        ));

        if frame_counter % rng.gen_range(1, 50) == 0 {
            // Fire a bullet
            commands.spawn(SpriteComponents {
                transform: *trans,
                sprite: Sprite::new(Vec2::new(5., 5.)),
                ..Default::default()
            });
            commands.with(Vel {
                x: rng.gen_range(-2., 2.),
                y: rng.gen_range(-2., 2.),
            });
            commands.with(Bullet::default());
        }
    }
}

fn bullet_lifetime(mut commands: Commands, mut query: Query<(Entity, &mut Bullet)>) {
    for (ent, mut bullet) in &mut query.iter() {
        bullet.alive_frames += 1;

        if bullet.alive_frames > 100 {
            commands.despawn(ent);
        }
    }
}

fn destroy_asteroids(
    mut commands: Commands,
    mut asteroids: Query<With<Asteroid, (Entity, &Transform, &Sprite)>>,
    mut bullets: Query<With<Bullet, (&Transform, &Sprite)>>,
) {
    for (a_ent, a_trans, a_sprite) in &mut asteroids.iter() {
        let a_pos = a_trans.translation();
        for (b_trans, b_sprite) in &mut bullets.iter() {
            let b_pos = b_trans.translation();

            // Naive: just take the x dimensions of both sprites and use assume they are perfect
            // circles with a radius of x
            let radius = (a_sprite.size.x() + b_sprite.size.x()) / 2.;
            let distance = (a_pos - b_pos).length();
            if radius > distance {
                commands.despawn(a_ent);
            }
        }
    }
}

fn destroy_ship(
    mut commands: Commands,
    #[cfg(not(headless))] mut materials: ResMut<Assets<ColorMaterial>>,
    mut asteroids: Query<With<Asteroid, (&Transform, &Sprite)>>,
    mut ships: Query<With<Ship, (Entity, &Transform, &Sprite)>>,
) {
    'ship: for (s_ent, s_trans, s_sprite) in &mut ships.iter() {
        let s_pos = s_trans.translation();

        for (a_trans, a_sprite) in &mut asteroids.iter() {
            let a_pos = a_trans.translation();

            // Detect collision
            let radius = (a_sprite.size.x() + s_sprite.size.x()) / 2.;
            let distance = (a_pos - s_pos).length();

            if radius > distance {
                commands.despawn(s_ent);

                // Respawn the ship
                spawn_ship(
                    &mut commands,
                    #[cfg(not(headless))]
                    &mut materials,
                );

                continue 'ship;
            }
        }
    }
}

#[derive(Default)]
struct FrameCount(usize);

fn exit_game(mut frame_count: Local<FrameCount>, mut exit_events: ResMut<Events<AppExit>>) {
    frame_count.0 += 1;

    if frame_count.0 > RUN_FOR_FRAMES {
        exit_events.send(AppExit);
    }
}

fn main() {
    // Create CPU cycle and instruction counters
    let mut counters = perf_event::Group::new().unwrap();
    let cycles = perf_event::Builder::new()
        .group(&mut counters)
        .kind(perf_event::events::Hardware::REF_CPU_CYCLES)
        .build()
        .unwrap();
    let instructions = perf_event::Builder::new()
        .group(&mut counters)
        .kind(perf_event::events::Hardware::INSTRUCTIONS)
        .build()
        .unwrap();

    fn build_app() -> App {
        // Create Bevy app builder
        let mut builder = App::build();

        // Add default plugins for non-headless builds
        #[cfg(not(headless))]
        builder.add_default_plugins().add_resource(WinitConfig {
            return_from_run: true,
        });

        #[cfg(headless)]
        builder.add_plugin(TypeRegistryPlugin::default());
        builder.add_plugin(CorePlugin::default());
        builder.add_plugin(TransformPlugin::default());

        // Add game systems
        builder
            .add_startup_system(setup.system())
            .add_system(move_system.system())
            .add_system(exit_game.system())
            .add_system(move_ship.system())
            .add_system(bullet_lifetime.system())
            .add_system(boundary_mirror.system())
            .add_system(destroy_asteroids.system())
            .add_system(destroy_ship.system());

        builder.app
    }

    let mut metrics = Metrics {
        iterations: Vec::with_capacity(ITERATIONS),
    };

    for _ in 0..ITERATIONS {
        #[allow(unused_mut)]
        let mut app = build_app();

        // Get current instant
        let instant = Instant::now();

        // Enable CPU counters
        counters.enable().unwrap();

        // Run the app
        #[cfg(not(headless))]
        app.run();

        // Manually run update when headless as there is no window to do it
        #[cfg(headless)]
        for _ in 0..=RUN_FOR_FRAMES {
            app.update();
        }

        // Disable CPU counters
        counters.disable().unwrap();

        // Get time
        let elapsed = instant.elapsed();

        // Record CPU metrics
        let counts = counters.read().unwrap();
        metrics.iterations.push(IterationMetrics {
            cpu_cycles: counts[&cycles],
            cpu_instructions: counts[&instructions],
            avg_frame_time_us: elapsed.as_micros() as f64 / RUN_FOR_FRAMES as f64,
        });

        // Reset CPU counters
        counters.reset().unwrap();
    }

    println!("{}", serde_json::to_string(&metrics).unwrap());
}
