# Bevy Benchmark Games

These are "games" that require no user imput and that can be run headless for use as benchmarks for the Bevy ECS and core systems. The goal is to create something that has the overall "shape" of a game to help invoke performance characteristics more close to a real game.

Current examples are an Asteroids-ish game:

```
cargo run --example asteroids
```

Or to run it headless:

```
cargo run --example asteroids --features headless
```
