use bevy::{
    app::AppExit,
    core_pipeline::bloom::Bloom,
    input::mouse::MouseMotion,
    pbr::{
        DistanceFog, FogFalloff, ScreenSpaceAmbientOcclusion,
        ScreenSpaceAmbientOcclusionQualityLevel,
    },
    prelude::*,
    render::{mesh::Indices, render_asset::RenderAssetUsages, render_resource::PrimitiveTopology},
    window::{CursorGrabMode, PrimaryWindow, WindowMode},
};
use std::collections::HashMap;
mod daylight;
mod settings;
mod sky;
mod streaming;
mod terrain;
#[cfg(test)]
mod tests;
mod water;

#[cfg(test)]
const WORLD_RADIUS: i32 = 64;
const WORLD_BOTTOM: i32 = -24;
const PLAYER_HEIGHT: f32 = 2.4;
const WALK_SPEED: f32 = 7.0;

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum BlockKind {
    Grass = 0,
    Dirt = 1,
    RoseStone = 2,
    LavenderStone = 3,
    DeepSoil = 4,
    Crystal = 5,
    RoseOre = 6,
    GoldOre = 7,
    Water = 8,
    Sand = 9,
    Wood = 10,
    Planks = 11,
    Leaves = 12,
    Snow = 14,
    Moonstone = 15,
}

#[derive(Resource, Default)]
struct WorldData {
    blocks: HashMap<IVec3, BlockKind>,
    max_y: i32,
}

#[derive(Component)]
struct WorldChunk(IVec2);

#[derive(Component)]
struct Player;

#[derive(Component)]
struct PlayerCamera;

#[derive(Component)]
struct Sway {
    phase: f32,
    amount: f32,
}

#[derive(Resource, Default)]
struct Look {
    pitch: f32,
    yaw: f32,
}

#[derive(Resource)]
struct AtlasMaterial(Handle<StandardMaterial>);

#[derive(Resource)]
struct FoliageAssets {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
}

#[derive(Resource)]
struct Palette {
    selected: usize,
}

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.65, 0.72, 0.86)))
        .insert_resource(AmbientLight {
            color: Color::srgb(0.8, 0.86, 1.0),
            brightness: 240.0,
        })
        .insert_resource(Look {
            pitch: -0.20,
            yaw: 0.35,
        })
        .insert_resource(Palette { selected: 0 })
        .init_resource::<streaming::Streaming>()
        .init_resource::<daylight::DayCycle>()
        .init_resource::<settings::Settings>()
        .insert_resource(bevy::pbr::DirectionalLightShadowMap { size: 4096 })
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "PRISM PETAL VALLEY ✦".into(),
                        resolution: (1440_f32, 900_f32).into(),
                        mode: WindowMode::Windowed,
                        resizable: true,
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins(bevy::diagnostic::FrameTimeDiagnosticsPlugin)
        .add_plugins(MaterialPlugin::<water::WaterMaterial>::default())
        .add_plugins(bevy::core_pipeline::experimental::taa::TemporalAntiAliasPlugin)
        .add_plugins(MaterialPlugin::<sky::SkyMaterial> {
            prepass_enabled: true,
            shadows_enabled: false,
            ..default()
        })
        .add_systems(Startup, setup)
        .add_systems(Startup, sky::setup)
        .add_systems(
            Update,
            (
                settings::update,
                settings::fps_counter,
                cursor_grab,
                mouse_look,
                player_movement,
                palette_input,
                sculpt_blocks,
                sway_foliage,
                streaming::update,
                daylight::update,
                sky::update,
                smoke_capture,
            )
                .chain(),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut water_materials: ResMut<Assets<water::WaterMaterial>>,
) {
    let mut world = WorldData::default();
    terrain::Generator { seed: world_seed() }.generate_region(
        &mut world,
        IVec2::ZERO,
        IVec2::splat(47),
    );
    let generator = terrain::Generator { seed: world_seed() };
    // Start above nearby slopes and crowns, not facing into a riverbank.
    let spawn_height = (12..=36)
        .flat_map(|x| (12..=36).map(move |z| (x, z)))
        .map(|(x, z)| generator.column(x, z).0)
        .max()
        .unwrap_or(terrain::SEA_LEVEL)
        .max(terrain::SEA_LEVEL) as f32
        + 9.;
    info!(
        "Generated seed {}: {} blocks",
        world_seed(),
        world.blocks.len()
    );
    let atlas: Handle<Image> = asset_server.load("textures/atlas.png");
    let material = materials.add(StandardMaterial {
        base_color_texture: Some(atlas),
        perceptual_roughness: 0.9,
        ..default()
    });
    spawn_chunk(&mut commands, &mut meshes, &material, &world);
    let foliage = create_foliage_assets(&mut meshes, &mut materials);
    spawn_foliage(&mut commands, &foliage, &world);
    commands.insert_resource(foliage);
    water::spawn(&mut commands, &mut meshes, &mut water_materials, &world);
    commands.insert_resource(AtlasMaterial(material));
    commands.insert_resource(world);

    commands.spawn((
        daylight::Sun,
        bevy::pbr::CascadeShadowConfigBuilder {
            maximum_distance: 128.,
            first_cascade_far_bound: 16.,
            ..default()
        }
        .build(),
        DirectionalLight {
            illuminance: 18_000.0,
            color: Color::srgb(1.0, 0.89, 0.8),
            shadows_enabled: true,
            shadow_depth_bias: 0.02,
            ..default()
        },
        Transform::from_xyz(-24., 32., 18.).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        daylight::Moon,
        DirectionalLight {
            illuminance: 1_500.0,
            color: Color::srgb(0.72, 0.78, 1.0),
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(18., 18., -20.).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        Camera3d::default(),
        Camera {
            hdr: true,
            ..default()
        },
        Msaa::Off,
        bevy::core_pipeline::experimental::taa::TemporalAntiAliasing::default(),
        bevy::pbr::ShadowFilteringMethod::Temporal,
        ScreenSpaceAmbientOcclusion {
            quality_level: ScreenSpaceAmbientOcclusionQualityLevel::High,
            constant_object_thickness: 0.55,
        },
        Bloom {
            intensity: 0.15,
            low_frequency_boost: 0.35,
            ..Bloom::NATURAL
        },
        DistanceFog {
            color: Color::srgb(0.65, 0.72, 0.86),
            directional_light_color: Color::srgba(1.0, 0.22, 0.55, 0.35),
            directional_light_exponent: 8.0,
            falloff: FogFalloff::Linear {
                start: 38.0,
                end: 64.0,
            },
        },
        Transform::from_xyz(24., spawn_height + PLAYER_HEIGHT, 24.)
            .with_rotation(Quat::from_euler(EulerRot::YXZ, 0.35, -0.20, 0.0)),
        PlayerCamera,
    ));
    commands.spawn((Player, Transform::from_xyz(24., spawn_height, 24.)));
    for (position, color) in [
        (Vec3::new(-8., 6., -5.), Color::srgb(1.0, 0.08, 0.55)),
        (Vec3::new(8., 7., 2.), Color::srgb(0.15, 0.8, 1.0)),
        (Vec3::new(-12., 5., 11.), Color::srgb(0.75, 0.25, 1.0)),
    ]
    .into_iter()
    {
        commands.spawn((
            PointLight {
                color,
                intensity: 1_500.0,
                range: 12.0,
                shadows_enabled: false,
                ..default()
            },
            Transform::from_translation(position),
        ));
    }
    settings::setup(&mut commands);
}

#[cfg(test)]
fn generate_world(world: &mut WorldData) {
    terrain::Generator {
        seed: terrain::DEFAULT_SEED,
    }
    .generate(world);
}

fn world_seed() -> u32 {
    static SEED: std::sync::OnceLock<u32> = std::sync::OnceLock::new();
    *SEED.get_or_init(|| {
        std::env::var("MC_SEED")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| {
                use std::hash::BuildHasher;
                std::collections::hash_map::RandomState::new()
                    .hash_one(std::time::SystemTime::now()) as u32
            })
    })
}

fn spawn_chunk(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    material: &Handle<StandardMaterial>,
    world: &WorldData,
) {
    for x in 0..=2 {
        for z in 0..=2 {
            let chunk = IVec2::new(x, z);
            commands.spawn((
                Mesh3d(meshes.add(build_chunk_mesh(world, chunk))),
                MeshMaterial3d(material.clone()),
                Transform::IDENTITY,
                WorldChunk(chunk),
            ));
        }
    }
}

fn create_foliage_assets(
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) -> FoliageAssets {
    let material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.95,
        // Cutout geometry must also be visible through the opposite side.
        cull_mode: None,
        double_sided: true,
        ..default()
    });
    let mesh = meshes.add(leaf_cluster_mesh());
    FoliageAssets { mesh, material }
}

fn spawn_foliage(commands: &mut Commands, assets: &FoliageAssets, world: &WorldData) {
    for (&grid, &kind) in &world.blocks {
        if kind != BlockKind::Planks {
            continue;
        }
        commands.spawn((
            Mesh3d(assets.mesh.clone()),
            MeshMaterial3d(assets.material.clone()),
            Transform::from_translation(grid.as_vec3()),
            streaming::Decoration(streaming::chunk_of(grid)),
            Sway {
                phase: (grid.x + grid.z) as f32,
                amount: 0.006,
            },
        ));
    }
}

// A stepped crown with actual openings, not transparent blended boxes.
// Sparse inset layers provide parallax through the outer leaf silhouettes.
// Both layers share one mesh/material, including their cutout shadows.
fn leaf_cluster_mesh() -> Mesh {
    use std::collections::HashSet;
    let mut blocks = HashSet::new();
    for y in -1_i32..=2 {
        let radius: i32 = if y < 1 { 2 } else { 1 };
        for x in -radius..=radius {
            for z in -radius..=radius {
                if x.abs() == radius && z.abs() == radius && y != 0 {
                    continue;
                }
                blocks.insert(IVec3::new(x, y, z));
            }
        }
    }
    let faces = [
        (IVec3::X, Vec3::new(1., 0., 0.), Vec3::Y, Vec3::Z),
        (-IVec3::X, Vec3::new(0., 0., 1.), Vec3::Y, -Vec3::Z),
        (IVec3::Y, Vec3::new(0., 1., 1.), Vec3::X, -Vec3::Z),
        (-IVec3::Y, Vec3::ZERO, Vec3::X, Vec3::Z),
        (IVec3::Z, Vec3::new(1., 0., 1.), Vec3::Y, -Vec3::X),
        (-IVec3::Z, Vec3::ZERO, Vec3::Y, Vec3::X),
    ];
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();
    for &block in &blocks {
        for (normal, origin, u, v) in faces {
            if blocks.contains(&(block + normal)) {
                continue;
            }
            for layer in 0..2 {
                for a in 0..6 {
                    for b in 0..6 {
                        let hash = (block.x * 17
                            + block.y * 29
                            + block.z * 43
                            + a * 13
                            + b * 7
                            + a * b * 3
                            + normal.x * 11
                            + normal.y * 23
                            + normal.z * 31
                            + layer * 37)
                            .rem_euclid(29);
                        // 55% coverage outside, 31% inside. Different masks leave
                        // sightlines through the crown instead of plugging each hole.
                        if hash < if layer == 0 { 13 } else { 20 } {
                            continue;
                        }
                        // Dark recesses and clustered blush highlights, not separate spikes.
                        let color = match hash.rem_euclid(19) {
                            0..=3 => Color::srgb(0.57, 0.20, 0.34),
                            4..=8 => Color::srgb(0.82, 0.36, 0.51),
                            9..=15 => Color::srgb(0.96, 0.55, 0.67),
                            _ => Color::srgb(1.0, 0.75, 0.81),
                        }
                        .to_linear()
                        .to_f32_array();
                        let first = positions.len() as u32;
                        for (du, dv) in [(0., 0.), (1., 0.), (1., 1.), (0., 1.)] {
                            let p = block.as_vec3() + origin
                                - normal.as_vec3() * (layer as f32 * 0.28)
                                + u * ((a as f32 + du) / 6.)
                                + v * ((b as f32 + dv) / 6.);
                            positions.push(p.to_array());
                            normals.push(normal.as_vec3().to_array());
                            colors.push(color);
                        }
                        indices.extend([first, first + 1, first + 2, first, first + 2, first + 3]);
                    }
                }
            }
        }
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
    .with_inserted_indices(Indices::U32(indices))
}

fn sway_foliage(time: Res<Time>, mut foliage: Query<(&mut Transform, &Sway)>) {
    for (mut transform, sway) in &mut foliage {
        let wave = (time.elapsed_secs() * 1.7 + sway.phase).sin() * sway.amount;
        transform.rotation = Quat::from_rotation_z(wave);
    }
}

fn build_chunk_mesh(world: &WorldData, chunk: IVec2) -> Mesh {
    let mut positions = Vec::<[f32; 3]>::new();
    let mut normals = Vec::<[f32; 3]>::new();
    let mut uvs = Vec::<[f32; 2]>::new();
    let mut indices = Vec::<u32>::new();
    let directions = [
        (
            IVec3::X,
            [1., 0., 0.],
            [[1., 0., 0.], [1., 1., 0.], [1., 1., 1.], [1., 0., 1.]],
        ),
        (
            -IVec3::X,
            [-1., 0., 0.],
            [[0., 0., 1.], [0., 1., 1.], [0., 1., 0.], [0., 0., 0.]],
        ),
        (
            IVec3::Y,
            [0., 1., 0.],
            [[0., 1., 1.], [1., 1., 1.], [1., 1., 0.], [0., 1., 0.]],
        ),
        (
            -IVec3::Y,
            [0., -1., 0.],
            [[0., 0., 0.], [1., 0., 0.], [1., 0., 1.], [0., 0., 1.]],
        ),
        (
            IVec3::Z,
            [0., 0., 1.],
            [[1., 0., 1.], [1., 1., 1.], [0., 1., 1.], [0., 0., 1.]],
        ),
        (
            -IVec3::Z,
            [0., 0., -1.],
            [[0., 0., 0.], [0., 1., 0.], [1., 1., 0.], [1., 0., 0.]],
        ),
    ];
    // One contiguous, padded chunk cache replaces six hash lookups per solid
    // voxel. Border samples retain exact cross-chunk face culling.
    let height = (world.max_y - WORLD_BOTTOM + 3) as usize;
    let stride_x = 18 * height;
    let mut cells = vec![None; 18 * stride_x];
    let origin = IVec3::new(chunk.x * 16 - 1, WORLD_BOTTOM - 1, chunk.y * 16 - 1);
    for x in 0..18 {
        for z in 0..18 {
            for y in 0..height {
                let p = origin + IVec3::new(x as i32, y as i32, z as i32);
                cells[x * stride_x + z * height + y] = world.blocks.get(&p).copied();
            }
        }
    }
    for x in 1..17 {
        for z in 1..17 {
            for y in 1..height - 1 {
                let index = x * stride_x + z * height + y;
                let Some(kind) = cells[index] else {
                    continue;
                };
                if matches!(kind, BlockKind::Leaves | BlockKind::Water) {
                    continue;
                }
                let grid = origin + IVec3::new(x as i32, y as i32, z as i32);
                let adjacent = [
                    index + stride_x,
                    index - stride_x,
                    index + 1,
                    index - 1,
                    index + height,
                    index - height,
                ];
                for (&(neighbor, normal, corners), neighbor_index) in
                    directions.iter().zip(adjacent)
                {
                    if cells[neighbor_index]
                        .is_some_and(|kind| !matches!(kind, BlockKind::Water | BlockKind::Leaves))
                    {
                        continue;
                    }
                    let first = positions.len() as u32;
                    for (i, corner) in corners.iter().enumerate() {
                        positions.push([
                            grid.x as f32 + corner[0],
                            grid.y as f32 + corner[1],
                            grid.z as f32 + corner[2],
                        ]);
                        normals.push(normal);
                        let tile = if kind == BlockKind::Grass && neighbor != IVec3::Y {
                            BlockKind::Dirt
                        } else {
                            kind
                        };
                        uvs.push(atlas_uv(tile, i));
                    }
                    indices.extend_from_slice(&[
                        first,
                        first + 1,
                        first + 2,
                        first,
                        first + 2,
                        first + 3,
                    ]);
                }
            }
        }
    }
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

fn atlas_uv(kind: BlockKind, corner: usize) -> [f32; 2] {
    let tile = kind as usize;
    let col = (tile % 4) as f32;
    let row = (tile / 4) as f32;
    let inset = 0.002;
    let min_x = col / 4.0 + inset;
    let max_x = (col + 1.0) / 4.0 - inset;
    let min_y = row / 4.0 + inset;
    let max_y = (row + 1.0) / 4.0 - inset;
    match corner {
        0 => [min_x, max_y],
        1 => [min_x, min_y],
        2 => [max_x, min_y],
        _ => [max_x, max_y],
    }
}

fn cursor_grab(
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    settings: Res<settings::Settings>,
    buttons: Query<&Interaction, With<Button>>,
) {
    if settings.open || buttons.iter().any(|i| *i != Interaction::None) {
        return;
    }
    if std::env::var_os("MC_CAPTURE").is_some() {
        return;
    }
    let Ok(mut window) = windows.get_single_mut() else {
        return;
    };
    if keys.just_pressed(KeyCode::Escape) || !window.focused {
        window.cursor_options.visible = true;
        window.cursor_options.grab_mode = CursorGrabMode::None;
    } else if mouse.just_pressed(MouseButton::Left) || mouse.just_pressed(MouseButton::Right) {
        window.cursor_options.visible = false;
        window.cursor_options.grab_mode = CursorGrabMode::Locked;
    }
}

fn mouse_look(
    mut motion: EventReader<MouseMotion>,
    mut look: ResMut<Look>,
    mut camera: Query<&mut Transform, With<PlayerCamera>>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    if std::env::var_os("MC_CAPTURE").is_some() {
        motion.clear();
        return;
    }
    let Ok(window) = windows.get_single() else {
        return;
    };
    if window.cursor_options.grab_mode != CursorGrabMode::Locked {
        motion.clear();
        return;
    }
    let mut delta = Vec2::ZERO;
    for event in motion.read() {
        delta += event.delta;
    }
    if delta == Vec2::ZERO {
        return;
    }
    look.yaw -= delta.x * 0.0025;
    look.pitch = (look.pitch - delta.y * 0.0025).clamp(-1.45, 1.45);
    if let Ok(mut transform) = camera.get_single_mut() {
        transform.rotation = Quat::from_euler(EulerRot::YXZ, look.yaw, look.pitch, 0.0);
    }
}

fn player_movement(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut player: Query<&mut Transform, With<Player>>,
    mut camera: Query<&mut Transform, (With<PlayerCamera>, Without<Player>)>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    if !windows.single().focused || windows.single().cursor_options.visible {
        return;
    }
    let Ok(mut player) = player.get_single_mut() else {
        return;
    };
    let Ok(mut cam) = camera.get_single_mut() else {
        return;
    };
    let forward = Vec3::new(cam.forward().x, 0., cam.forward().z).normalize_or_zero();
    let right = Vec3::new(cam.right().x, 0., cam.right().z).normalize_or_zero();
    let mut input = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) {
        input += forward;
    }
    if keys.pressed(KeyCode::KeyS) {
        input -= forward;
    }
    if keys.pressed(KeyCode::KeyA) {
        input -= right;
    }
    if keys.pressed(KeyCode::KeyD) {
        input += right;
    }
    if keys.pressed(KeyCode::Space) {
        input += Vec3::Y;
    }
    if keys.pressed(KeyCode::ControlLeft) {
        input -= Vec3::Y;
    }
    if input != Vec3::ZERO {
        let speed = if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
            WALK_SPEED * 1.65
        } else {
            WALK_SPEED
        };
        player.translation += input.normalize() * speed * time.delta_secs();
    }
    cam.translation = player.translation + Vec3::Y * PLAYER_HEIGHT;
}

fn palette_input(keys: Res<ButtonInput<KeyCode>>, mut palette: ResMut<Palette>) {
    for (index, key) in [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
    ]
    .into_iter()
    .enumerate()
    {
        if keys.just_pressed(key) {
            palette.selected = index;
        }
    }
    if keys.just_pressed(KeyCode::KeyQ) {
        palette.selected = (palette.selected + 3) % 4;
    }
    if keys.just_pressed(KeyCode::KeyE) {
        palette.selected = (palette.selected + 1) % 4;
    }
}

#[allow(clippy::too_many_arguments)] // Bevy injects these system parameters.
fn sculpt_blocks(
    mouse: Res<ButtonInput<MouseButton>>,
    palette: Res<Palette>,
    material: Res<AtlasMaterial>,
    mut world: ResMut<WorldData>,
    camera: Query<&GlobalTransform, With<PlayerCamera>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    chunks: Query<(Entity, &WorldChunk)>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut captured_last_frame: Local<bool>,
    mut streaming: ResMut<streaming::Streaming>,
) {
    let captured = windows.single().focused && !windows.single().cursor_options.visible;
    let can_edit = captured && *captured_last_frame;
    *captured_last_frame = captured;
    if !can_edit {
        return;
    }
    let add = mouse.just_pressed(MouseButton::Right);
    let remove = mouse.just_pressed(MouseButton::Left);
    if !add && !remove {
        return;
    }
    let Ok(cam) = camera.get_single() else { return };
    let mut previous = cam.translation().floor().as_ivec3();
    let mut hit = None;
    for step in 1..=140 {
        let cell = (cam.translation() + cam.forward() * (step as f32 * 0.05))
            .floor()
            .as_ivec3();
        if world
            .blocks
            .get(&cell)
            .is_some_and(|kind| *kind != BlockKind::Water && *kind != BlockKind::Leaves)
        {
            hit = Some(if add { previous } else { cell });
            break;
        }
        previous = cell;
    }
    let Some(target) = hit else { return };
    if target.y <= WORLD_BOTTOM || !streaming.loaded.contains(&streaming::chunk_of(target)) {
        return;
    }
    if add {
        world.max_y = world.max_y.max(target.y);
        let kind = match palette.selected {
            0 => BlockKind::Grass,
            1 => BlockKind::Crystal,
            2 => BlockKind::RoseOre,
            _ => BlockKind::GoldOre,
        };
        world.blocks.insert(target, kind);
    } else {
        world.blocks.remove(&target);
    }
    streaming
        .edits
        .insert(target, world.blocks.get(&target).copied());
    for (entity, chunk) in &chunks {
        if [IVec3::ZERO, IVec3::X, -IVec3::X, IVec3::Z, -IVec3::Z]
            .iter()
            .any(|offset| {
                let p = target + *offset;
                IVec2::new(p.x.div_euclid(16), p.z.div_euclid(16)) == chunk.0
            })
        {
            commands.entity(entity).despawn_recursive();
            commands.spawn((
                Mesh3d(meshes.add(build_chunk_mesh(&world, chunk.0))),
                MeshMaterial3d(material.0.clone()),
                Transform::IDENTITY,
                WorldChunk(chunk.0),
            ));
        }
    }
}

// Optional reproducible runtime capture: MC_CAPTURE=/tmp/prism.png cargo run.
fn smoke_capture(
    mut commands: Commands,
    time: Res<Time>,
    mut done: Local<bool>,
    mut exit: EventWriter<AppExit>,
    diagnostics: Res<bevy::diagnostic::DiagnosticsStore>,
) {
    let Ok(path) = std::env::var("MC_CAPTURE") else {
        return;
    };
    if time.elapsed_secs() > 12.0 && !*done {
        if let Some(fps) = diagnostics
            .get(&bevy::diagnostic::FrameTimeDiagnosticsPlugin::FPS)
            .and_then(|d| d.smoothed())
        {
            info!("Capture warm-frame FPS: {fps:.1}");
        }
        commands
            .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
            .observe(bevy::render::view::screenshot::save_to_disk(path));
        *done = true;
    }
    if time.elapsed_secs() > 16.0 {
        exit.send(AppExit::Success);
    }
}
