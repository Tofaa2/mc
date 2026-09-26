use super::*;
use bevy::render::render_resource::{AsBindGroup, ShaderRef};

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct SkyMaterial {
    #[uniform(0)]
    pub sun: Vec4,
}
impl Material for SkyMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/sky.wgsl".into()
    }
}
#[derive(Component)]
pub struct Sky;

pub fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<SkyMaterial>>,
) {
    let mut mesh = Sphere::new(500.).mesh().uv(48, 24);
    // Reverse winding so the sphere is visible from inside.
    if let Some(Indices::U32(indices)) = mesh.indices_mut() {
        for tri in indices.as_chunks_mut::<3>().0 {
            tri.swap(1, 2);
        }
    }
    commands.spawn((
        Sky,
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(materials.add(SkyMaterial { sun: Vec4::ZERO })),
        Transform::IDENTITY,
        bevy::pbr::NotShadowCaster,
        bevy::pbr::NotShadowReceiver,
    ));
}

pub fn update(
    cycle: Res<daylight::DayCycle>,
    time: Res<Time>,
    camera: Query<&Transform, (With<PlayerCamera>, Without<Sky>)>,
    mut sky: Query<(&mut Transform, &MeshMaterial3d<SkyMaterial>), With<Sky>>,
    mut materials: ResMut<Assets<SkyMaterial>>,
) {
    let Ok(camera) = camera.get_single() else {
        return;
    };
    let angle = cycle.phase * std::f32::consts::TAU;
    let sun = Vec3::new(angle.cos(), angle.sin(), 0.35).normalize();
    for (mut transform, handle) in &mut sky {
        transform.translation = camera.translation;
        if let Some(material) = materials.get_mut(&handle.0) {
            material.sun = sun.extend(time.elapsed_secs());
        }
    }
}
