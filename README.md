# Prism Petal Valley

A tiny procedural Minecraft-inspired voxel dreamscape written in Rust with Bevy and wgpu.

The current vertical slice includes:

- Streamed seeded terrain: meadows, cherry groves, rocky highlands, snowy peaks, river valleys and beaches
- Underground caves, deep strata and clustered ores down to Y=-24
- Exposed-face meshes in 16×16 chunks; edits rebuild only affected chunks
- Pink-lavender distance fog, bloom, SSAO, and candy-colored lighting for depth without overloading the GPU
- GPU-animated translucent water with ripple normals and a Fresnel sky sheen (not real-time reflections)
- Shared stepped voxel canopies with real leaf gaps, inset foliage layers, pink shading, and subtle sway
- Ten-minute day/night cycle with a moving shadow-casting sun, moonlight, warm twilight, and matching sky/fog
- First-person creative flight and mouse look
- Right-click block placement with four palette materials; left-click mining
- Compact settings panel for time presets, cycle pause/resume, and fog distance
- Procedural day/night sky with sun, moon, stars, and drifting clouds
- TAA, temporal shadow filtering, and 4096-pixel directional shadow maps

Run it with:

```sh
cargo run
```

Every launch picks a random seed, shown in the startup log.
Use `MC_SEED=12345 cargo run` to revisit a reproducible world.
Terrain uses warped, multi-octave noise; trees have biome-dependent density and
varying trunk heights. Fog presets (64/96/128 blocks) also select the chunk loading
radius; chunks unload one chunk beyond it, including water and foliage. Terrain generation is budgeted to one
chunk per frame; rapid travel may briefly outpace loading. There is no fixed
horizontal boundary (very distant coordinates remain subject to float precision).
Block edits survive unloading/reloading **during this session**, but are not saved
to disk. The seed regenerates terrain, not previous edits.

Use `MC_DAY_SECONDS=120 cargo run` for a two-minute cycle (default 600 seconds).

Click the world to capture the mouse; Escape or Tab opens settings and releases it.
The Settings button also opens the panel. Choose Dawn, Noon, Sunset, or Night,
pause/resume the cycle, or increase fog distance (which loads more terrain).
WASD moves, Shift sprints,
Space flies up, and left Ctrl flies down. Left-click mines and right-click places
within seven blocks. 1–4 selects a block; Q/E cycles the palette.
This is creative flight, without survival gravity or collision.

Dependencies are optimized in development builds; the first build takes longer.
On NixOS use `nix develop` first, or the supplied direnv environment.

Run `cargo test` for geometry/world tests. `MC_CAPTURE=/tmp/prism.png cargo run`
saves a runtime screenshot, logs smoothed FPS after warmup, and exits automatically.

CPU meshing benchmark (fixed seed, 180 meshes):
`cargo test benchmark_chunk_meshing -- --ignored --nocapture`.
