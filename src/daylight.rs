use super::*;

#[derive(Component)]
pub struct Sun;
#[derive(Component)]
pub struct Moon;

#[derive(Resource)]
pub struct DayCycle {
    pub phase: f32,
    seconds: f32,
    pub paused: bool,
}
impl Default for DayCycle {
    fn default() -> Self {
        let seconds = std::env::var("MC_DAY_SECONDS")
            .ok()
            .and_then(|s| s.parse::<f32>().ok())
            .filter(|s| s.is_finite() && *s >= 10.)
            .unwrap_or(600.);
        Self {
            phase: 0.20,
            seconds,
            paused: false,
        }
    }
}

fn daylight(phase: f32) -> f32 {
    ((phase * std::f32::consts::TAU).sin() * 3. + 0.25).clamp(0., 1.)
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)] // Disjoint Bevy light queries.
pub fn update(
    time: Res<Time>,
    mut cycle: ResMut<DayCycle>,
    mut sun: Query<(&mut DirectionalLight, &mut Transform), (With<Sun>, Without<Moon>)>,
    mut moon: Query<(&mut DirectionalLight, &mut Transform), (With<Moon>, Without<Sun>)>,
    mut ambient: ResMut<AmbientLight>,
    mut clear: ResMut<ClearColor>,
    mut fog: Query<&mut DistanceFog>,
) {
    if !cycle.paused {
        cycle.phase = (cycle.phase + time.delta_secs() / cycle.seconds).rem_euclid(1.);
    }
    let angle = cycle.phase * std::f32::consts::TAU;
    let height = angle.sin();
    let day = daylight(cycle.phase);
    let twilight = (1. - height.abs() * 5.).clamp(0., 1.);
    let direction = Vec3::new(angle.cos(), height, 0.35).normalize();
    if let Ok((mut light, mut transform)) = sun.get_single_mut() {
        light.illuminance = 18000. * height.max(0.);
        light.color = Color::srgb(1., 0.89 - 0.4 * twilight, 0.8 - 0.45 * twilight);
        *transform = Transform::from_translation(direction * 100.).looking_at(Vec3::ZERO, Vec3::Y);
    }
    if let Ok((mut light, mut transform)) = moon.get_single_mut() {
        light.illuminance = 1100. * (-height).max(0.);
        light.color = Color::srgb(0.55, 0.65, 1.);
        *transform = Transform::from_translation(-direction * 100.).looking_at(Vec3::ZERO, Vec3::Y);
    }
    ambient.brightness = 35. + day * 205.;
    let sky = Vec3::new(0.025, 0.035, 0.095).lerp(Vec3::new(0.65, 0.72, 0.86), day)
        + Vec3::new(0.22, 0.035, 0.06) * twilight;
    let color = Color::srgb(sky.x, sky.y, sky.z);
    clear.0 = color;
    for mut fog in &mut fog {
        fog.color = color;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn day_and_night_have_distinct_light_levels() {
        assert_eq!(daylight(0.25), 1.);
        assert_eq!(daylight(0.75), 0.);
        assert!(daylight(0.) > 0. && daylight(0.) < 1.);
    }
}
