use super::*;
use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::render::render_resource::{AsBindGroup, ShaderRef};

pub type WaterMaterial = ExtendedMaterial<StandardMaterial, WaterExtension>;

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct WaterExtension {
    #[uniform(100)]
    tint: Vec4,
}

impl MaterialExtension for WaterExtension {
    fn vertex_shader() -> ShaderRef {
        "shaders/water.wgsl".into()
    }
    fn fragment_shader() -> ShaderRef {
        "shaders/water.wgsl".into()
    }
}

pub fn spawn(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<WaterMaterial>,
    world: &WorldData,
) {
    let material = materials.add(WaterMaterial {
        base: StandardMaterial {
            base_color: Color::srgba(0.23, 0.65, 0.72, 0.78),
            alpha_mode: AlphaMode::Blend,
            perceptual_roughness: 0.18,
            metallic: 0.18,
            reflectance: 0.65,
            cull_mode: None,
            ..default()
        },
        extension: WaterExtension {
            tint: Vec4::new(0.55, 0.72, 0.9, 1.0),
        },
    });
    let chunks: std::collections::HashSet<_> = world
        .blocks
        .keys()
        .map(|p| streaming::chunk_of(*p))
        .collect();
    for chunk in chunks {
        let (cx, cz) = (chunk.x, chunk.y);
        let mut positions = Vec::new();
        let mut indices = Vec::new();
        for x in cx * 16..cx * 16 + 16 {
            for z in cz * 16..cz * 16 + 16 {
                for y in [terrain::SEA_LEVEL] {
                    let p = IVec3::new(x, y, z);
                    if world.blocks.get(&p) != Some(&BlockKind::Water)
                        || world.blocks.contains_key(&(p + IVec3::Y))
                    {
                        continue;
                    }
                    let first = positions.len() as u32;
                    for [dx, dz] in [[0., 1.], [1., 1.], [1., 0.], [0., 0.]] {
                        positions.push([x as f32 + dx, y as f32 + 0.88, z as f32 + dz]);
                    }
                    indices.extend([first, first + 1, first + 2, first, first + 2, first + 3]);
                }
            }
        }
        if positions.is_empty() {
            continue;
        }
        let mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions.clone())
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0., 1., 0.]; positions.len()])
        .with_inserted_indices(Indices::U32(indices));
        commands.spawn((
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(material.clone()),
            Transform::IDENTITY,
            bevy::pbr::NotShadowCaster,
            streaming::Decoration(chunk),
        ));
    }
}
