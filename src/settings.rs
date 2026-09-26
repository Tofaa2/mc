use super::*;
use bevy::core_pipeline::experimental::taa::TemporalAntiAliasing;

#[derive(Resource)]
pub struct Settings {
    pub open: bool,
    pub fog_end: f32,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            open: false,
            fog_end: 96.,
        }
    }
}
#[derive(Component)]
pub struct Panel;
#[derive(Component, Clone, Copy)]
pub enum Action {
    Toggle,
    Dawn,
    Noon,
    Sunset,
    Night,
    Pause,
    Near,
    Medium,
    Far,
}
#[derive(Component)]
pub struct Status;
#[derive(Component)]
pub struct FpsCounter;

pub fn setup(commands: &mut Commands) {
    commands.spawn((
        FpsCounter,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(16.),
            top: Val::Px(16.),
            padding: UiRect::axes(Val::Px(10.), Val::Px(6.)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.04, 0.02, 0.06, 0.8)),
        Text::new("FPS --"),
        TextFont {
            font_size: 14.,
            ..default()
        },
        TextColor(Color::srgb(1., 0.84, 0.91)),
    ));
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            right: Val::Px(20.),
            top: Val::Px(20.),
            ..default()
        })
        .with_children(|p| {
            button(p, "Settings", Action::Toggle);
        });
    commands
        .spawn((
            Panel,
            Node {
                display: Display::None,
                position_type: PositionType::Absolute,
                right: Val::Px(20.),
                top: Val::Px(70.),
                width: Val::Px(300.),
                padding: UiRect::all(Val::Px(20.)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(12.),
                ..default()
            },
            BackgroundColor(Color::srgba(0.045, 0.025, 0.07, 0.96)),
            BorderRadius::all(Val::Px(12.)),
        ))
        .with_children(|p| {
            p.spawn((
                Text::new("WORLD SETTINGS"),
                TextFont {
                    font_size: 18.,
                    ..default()
                },
                TextColor(Color::srgb(1., 0.7, 0.83)),
            ));
            p.spawn((
                Status,
                Text::new(""),
                TextFont {
                    font_size: 14.,
                    ..default()
                },
            ));
            for (label, action) in [
                ("Dawn", Action::Dawn),
                ("Noon", Action::Noon),
                ("Sunset", Action::Sunset),
                ("Night", Action::Night),
                ("Pause / resume cycle", Action::Pause),
                ("Fog: near / 64 blocks", Action::Near),
                ("Fog: medium / 96 blocks", Action::Medium),
                ("Fog: far / 128 blocks", Action::Far),
                ("Close / Esc", Action::Toggle),
            ] {
                button(p, label, action);
            }
        });
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.),
            top: Val::Percent(50.),
            width: Val::Px(3.),
            height: Val::Px(3.),
            ..default()
        },
        BackgroundColor(Color::WHITE),
    ));
}

fn button(parent: &mut ChildBuilder, label: &str, action: Action) {
    parent
        .spawn((
            Button,
            action,
            Node {
                padding: UiRect::axes(Val::Px(14.), Val::Px(9.)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.19, 0.075, 0.14)),
            BorderRadius::all(Val::Px(6.)),
        ))
        .with_children(|p| {
            p.spawn((
                Text::new(label),
                TextFont {
                    font_size: 14.,
                    ..default()
                },
                TextColor(Color::srgb(1., 0.84, 0.91)),
            ));
        });
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn update(
    keys: Res<ButtonInput<KeyCode>>,
    mut settings: ResMut<Settings>,
    mut cycle: ResMut<daylight::DayCycle>,
    mut buttons: Query<
        (&Interaction, &Action, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
    mut panels: Query<&mut Node, With<Panel>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mut status: Query<&mut Text, With<Status>>,
    mut fog: Query<&mut DistanceFog>,
    mut taa: Query<&mut TemporalAntiAliasing>,
) {
    if keys.just_pressed(KeyCode::Escape) || keys.just_pressed(KeyCode::Tab) {
        settings.open = !settings.open;
    }
    for (interaction, action, mut color) in &mut buttons {
        color.0 = if *interaction == Interaction::None {
            Color::srgb(0.19, 0.075, 0.14)
        } else {
            Color::srgb(0.39, 0.14, 0.27)
        };
        if *interaction != Interaction::Pressed {
            continue;
        }
        match action {
            Action::Toggle => settings.open = !settings.open,
            Action::Dawn => cycle.phase = 0.02,
            Action::Noon => cycle.phase = 0.25,
            Action::Sunset => cycle.phase = 0.48,
            Action::Night => cycle.phase = 0.75,
            Action::Pause => cycle.paused = !cycle.paused,
            Action::Near => settings.fog_end = 64.,
            Action::Medium => settings.fog_end = 96.,
            Action::Far => settings.fog_end = 128.,
        }
        for mut taa in &mut taa {
            taa.reset = true;
        }
    }
    for mut panel in &mut panels {
        let display = if settings.open {
            Display::Flex
        } else {
            Display::None
        };
        if panel.display != display {
            panel.display = display;
        }
    }
    if settings.open
        && let Ok(mut window) = windows.get_single_mut()
    {
        window.cursor_options.visible = true;
        window.cursor_options.grab_mode = CursorGrabMode::None;
    }
    for mut fog in &mut fog {
        if matches!(fog.falloff,FogFalloff::Linear {end,..} if end == settings.fog_end) {
            continue;
        }
        fog.falloff = FogFalloff::Linear {
            start: settings.fog_end * 0.6,
            end: settings.fog_end,
        };
    }
    for mut text in &mut status {
        if !settings.open {
            continue;
        }
        let hour = (cycle.phase * 24. + 6.).rem_euclid(24.);
        let label = format!(
            "{:02}:{:02}  /  {}\nFog distance: {} blocks",
            hour as u32,
            ((hour.fract()) * 60.) as u32,
            if cycle.paused { "Paused" } else { "Cycling" },
            settings.fog_end as u32
        );
        if text.0 != label {
            text.0 = label;
        }
    }
}

pub fn fps_counter(
    time: Res<Time<Real>>,
    diagnostics: Res<bevy::diagnostic::DiagnosticsStore>,
    mut elapsed: Local<f32>,
    mut labels: Query<&mut Text, With<FpsCounter>>,
) {
    *elapsed += time.delta_secs();
    if *elapsed < 0.25 {
        return;
    }
    *elapsed = 0.;
    let Some(fps) = diagnostics
        .get(&bevy::diagnostic::FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
    else {
        return;
    };
    let ms = diagnostics
        .get(&bevy::diagnostic::FrameTimeDiagnosticsPlugin::FRAME_TIME)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.);
    let label = format!("{fps:.0} FPS  /  {ms:.1} ms");
    for mut text in &mut labels {
        if text.0 != label {
            text.0.clone_from(&label);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn buttons_set_time_fog_and_release_cursor() {
        let mut app = App::new();
        app.insert_resource(ButtonInput::<KeyCode>::default())
            .insert_resource(Settings {
                open: true,
                fog_end: 96.,
            })
            .init_resource::<daylight::DayCycle>()
            .add_systems(Update, update);
        app.world_mut().spawn((PrimaryWindow, Window::default()));
        app.world_mut().spawn((Panel, Node::default()));
        app.world_mut()
            .spawn((DistanceFog::default(), TemporalAntiAliasing::default()));
        let button = app
            .world_mut()
            .spawn((
                Button,
                Interaction::Pressed,
                Action::Night,
                BackgroundColor::default(),
            ))
            .id();
        app.update();
        assert_eq!(app.world().resource::<daylight::DayCycle>().phase, 0.75);
        assert!(
            app.world_mut()
                .query::<&Window>()
                .single(app.world())
                .cursor_options
                .visible
        );
        app.world_mut()
            .entity_mut(button)
            .insert((Interaction::None, Action::Far));
        app.update();
        app.world_mut()
            .entity_mut(button)
            .insert(Interaction::Pressed);
        app.update();
        assert_eq!(app.world().resource::<Settings>().fog_end, 128.);
        assert!(
            app.world_mut()
                .query::<&TemporalAntiAliasing>()
                .single(app.world())
                .reset
        );
    }
}
