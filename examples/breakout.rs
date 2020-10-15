use std::time::Instant;

use bevy::{
    app::AppExit,
    core::CorePlugin,
    prelude::*,
    render::pass::ClearColor,
    sprite::collide_aabb::{collide, Collision},
    type_registry::TypeRegistryPlugin,
};

#[cfg(not(headless))]
use bevy::winit::WinitConfig;

use bevy_benchmark_games::{metrics::IterationMetrics, metrics::Metrics, random::FakeRand};
use rand::Rng;

#[cfg(headless)]
const RUN_FOR_FRAMES: usize = 300;
#[cfg(not(headless))]
const RUN_FOR_FRAMES: usize = 400;

#[cfg(headless)]
const ITERATIONS: usize = 200;
#[cfg(not(headless))]
const ITERATIONS: usize = 2;

/// An implementation of the classic game "Breakout"
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
        let mut builder = App::build();

        #[cfg(not(headless))]
        builder.add_default_plugins().add_resource(WinitConfig {
            return_from_run: true,
        });

        #[cfg(headless)]
        builder
            .add_plugin(TypeRegistryPlugin::default())
            .add_plugin(CorePlugin::default())
            .add_plugin(TransformPlugin::default());

        builder
            .add_resource(Scoreboard { score: 0 })
            .add_resource(ClearColor(Color::rgb(0.7, 0.7, 0.7)))
            .add_startup_system(setup.system())
            .add_system(paddle_movement_system.system())
            .add_system(ball_collision_system.system())
            .add_system(ball_movement_system.system())
            .add_system(scoreboard_system.system())
            .add_system(exit_game.system());

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

        #[cfg(not(headless))]
        app.run();

        #[cfg(headless)]
        for _ in 0..RUN_FOR_FRAMES {
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

    // Output metrics to be consumed by benchmarking harness
    println!("{}", serde_json::to_string(&metrics).unwrap());
}

struct Paddle {
    speed: f32,
}

struct Ball {
    velocity: Vec3,
}

struct Scoreboard {
    score: usize,
}

enum Collider {
    Solid,
    Scorable,
}

fn setup(
    mut commands: Commands,
    #[cfg(not(headless))] mut materials: ResMut<Assets<ColorMaterial>>,
    #[cfg(not(headless))] asset_server: Res<AssetServer>,
) {
    // Add the game's entities to our world
    commands
        // cameras
        .spawn(Camera2dComponents::default())
        .spawn(UiCameraComponents::default())
        // paddle
        .spawn(SpriteComponents {
            #[cfg(not(headless))]
            material: materials.add(Color::rgb(0.2, 0.2, 0.8).into()),
            transform: Transform::from_translation(Vec3::new(0.0, -215.0, 0.0)),
            sprite: Sprite::new(Vec2::new(120.0, 30.0)),
            ..Default::default()
        })
        .with(Paddle { speed: 500.0 })
        .with(Collider::Solid)
        // ball
        .spawn(SpriteComponents {
            #[cfg(not(headless))]
            material: materials.add(Color::rgb(0.8, 0.2, 0.2).into()),
            transform: Transform::from_translation(Vec3::new(0.0, -50.0, 1.0)),
            sprite: Sprite::new(Vec2::new(30.0, 30.0)),
            ..Default::default()
        })
        .with(Ball {
            velocity: 400.0 * Vec3::new(0.5, -0.5, 0.0).normalize(),
        });

    #[cfg(not(headless))]
    commands
        // scoreboard
        .spawn(TextComponents {
            text: Text {
                font: asset_server.load("assets/fonts/FiraSans-Bold.ttf").unwrap(),
                value: "Score:".to_string(),
                style: TextStyle {
                    color: Color::rgb(0.2, 0.2, 0.8),
                    font_size: 40.0,
                },
            },
            style: Style {
                position_type: PositionType::Absolute,
                position: Rect {
                    top: Val::Px(5.0),
                    left: Val::Px(5.0),
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        });

    // Add walls
    #[cfg(not(headless))]
    let wall_material = materials.add(Color::rgb(0.5, 0.5, 0.5).into());
    let wall_thickness = 10.0;
    let bounds = Vec2::new(900.0, 600.0);

    commands
        // left
        .spawn(SpriteComponents {
            #[cfg(not(headless))]
            material: wall_material,
            transform: Transform::from_translation(Vec3::new(-bounds.x() / 2.0, 0.0, 0.0)),
            sprite: Sprite::new(Vec2::new(wall_thickness, bounds.y() + wall_thickness)),
            ..Default::default()
        })
        .with(Collider::Solid)
        // right
        .spawn(SpriteComponents {
            #[cfg(not(headless))]
            material: wall_material,
            transform: Transform::from_translation(Vec3::new(bounds.x() / 2.0, 0.0, 0.0)),
            sprite: Sprite::new(Vec2::new(wall_thickness, bounds.y() + wall_thickness)),
            ..Default::default()
        })
        .with(Collider::Solid)
        // bottom
        .spawn(SpriteComponents {
            #[cfg(not(headless))]
            material: wall_material,
            transform: Transform::from_translation(Vec3::new(0.0, -bounds.y() / 2.0, 0.0)),
            sprite: Sprite::new(Vec2::new(bounds.x() + wall_thickness, wall_thickness)),
            ..Default::default()
        })
        .with(Collider::Solid)
        // top
        .spawn(SpriteComponents {
            #[cfg(not(headless))]
            material: wall_material,
            transform: Transform::from_translation(Vec3::new(0.0, bounds.y() / 2.0, 0.0)),
            sprite: Sprite::new(Vec2::new(bounds.x() + wall_thickness, wall_thickness)),
            ..Default::default()
        })
        .with(Collider::Solid);

    // Add bricks
    let brick_rows = 4;
    let brick_columns = 5;
    let brick_spacing = 20.0;
    let brick_size = Vec2::new(150.0, 30.0);
    let bricks_width = brick_columns as f32 * (brick_size.x() + brick_spacing) - brick_spacing;
    // center the bricks and move them up a bit
    let bricks_offset = Vec3::new(-(bricks_width - brick_size.x()) / 2.0, 100.0, 0.0);

    for row in 0..brick_rows {
        let y_position = row as f32 * (brick_size.y() + brick_spacing);
        for column in 0..brick_columns {
            let brick_position = Vec3::new(
                column as f32 * (brick_size.x() + brick_spacing),
                y_position,
                0.0,
            ) + bricks_offset;
            commands
                // brick
                .spawn(SpriteComponents {
                    #[cfg(not(headless))]
                    material: materials.add(Color::rgb(0.2, 0.2, 0.8).into()),
                    sprite: Sprite::new(brick_size),
                    transform: Transform::from_translation(brick_position),
                    ..Default::default()
                })
                .with(Collider::Scorable);
        }
    }
}

#[derive(Default)]
struct FrameCount(usize);

fn exit_game(mut state: Local<FrameCount>, mut exit_events: ResMut<Events<AppExit>>) {
    state.0 += 1;

    if state.0 > RUN_FOR_FRAMES {
        exit_events.send(AppExit);
    }
}

#[derive(Default)]
struct RngState {
    rng: FakeRand,
}

fn paddle_movement_system(
    mut state: Local<RngState>,
    time: Res<Time>,
    mut query: Query<(&Paddle, &mut Transform)>,
) {
    for (paddle, mut transform) in &mut query.iter() {
        let mut direction = 0.0;

        if state.rng.gen::<bool>() {
            direction -= 1.0;
        } else {
            direction += 1.0;
        }

        let translation = transform.translation_mut();
        // move the paddle horizontally
        *translation.x_mut() += time.delta_seconds * direction * paddle.speed;
        // bound the paddle within the walls
        *translation.x_mut() = translation.x().min(380.0).max(-380.0);
    }
}

fn ball_movement_system(time: Res<Time>, mut ball_query: Query<(&Ball, &mut Transform)>) {
    // clamp the timestep to stop the ball from escaping when the game starts
    let delta_seconds = f32::min(0.2, time.delta_seconds);

    for (ball, mut transform) in &mut ball_query.iter() {
        transform.translate(ball.velocity * delta_seconds);
    }
}

fn scoreboard_system(scoreboard: Res<Scoreboard>, mut query: Query<&mut Text>) {
    for mut text in &mut query.iter() {
        text.value = format!("Score: {}", scoreboard.score);
    }
}

fn ball_collision_system(
    mut commands: Commands,
    mut scoreboard: ResMut<Scoreboard>,
    mut ball_query: Query<(&mut Ball, &Transform, &Sprite)>,
    mut collider_query: Query<(Entity, &Collider, &Transform, &Sprite)>,
) {
    for (mut ball, ball_transform, sprite) in &mut ball_query.iter() {
        let ball_size = sprite.size;
        let velocity = &mut ball.velocity;

        // check collision with walls
        for (collider_entity, collider, transform, sprite) in &mut collider_query.iter() {
            let collision = collide(
                ball_transform.translation(),
                ball_size,
                transform.translation(),
                sprite.size,
            );
            if let Some(collision) = collision {
                // scorable colliders should be despawned and increment the scoreboard on collision
                if let Collider::Scorable = *collider {
                    scoreboard.score += 1;
                    commands.despawn(collider_entity);
                }

                // reflect the ball when it collides
                let mut reflect_x = false;
                let mut reflect_y = false;

                // only reflect if the ball's velocity is going in the opposite direction of the collision
                match collision {
                    Collision::Left => reflect_x = velocity.x() > 0.0,
                    Collision::Right => reflect_x = velocity.x() < 0.0,
                    Collision::Top => reflect_y = velocity.y() < 0.0,
                    Collision::Bottom => reflect_y = velocity.y() > 0.0,
                }

                // reflect velocity on the x-axis if we hit something on the x-axis
                if reflect_x {
                    *velocity.x_mut() = -velocity.x();
                }

                // reflect velocity on the y-axis if we hit something on the y-axis
                if reflect_y {
                    *velocity.y_mut() = -velocity.y();
                }

                break;
            }
        }
    }
}
