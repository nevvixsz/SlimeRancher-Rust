//! Day/night sky color and directional sun lighting synced to `TimeDirector`.

use std::f32::consts::PI;

use bevy::prelude::*;
use sr_sim::TimeDirector;

use crate::world_sync::SimResource;

#[derive(Component)]
pub struct SunLight;

pub fn setup_sky_and_light(mut commands: Commands) {
    commands.insert_resource(ClearColor(Color::srgb(0.47, 0.78, 0.96)));
    commands.insert_resource(AmbientLight {
        color: Color::srgb(0.95, 0.97, 1.0),
        brightness: 450.0,
    });

    commands.spawn((
        DirectionalLight {
            illuminance: 14_000.0,
            shadows_enabled: std::env::var_os("SR_SHADOWS").is_some(),
            ..default()
        },
        Transform::from_xyz(25.0, 45.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
        SunLight,
    ));
}

pub fn update_sky_and_light(
    sim: Res<SimResource>,
    mut clear: ResMut<ClearColor>,
    mut ambient: ResMut<AmbientLight>,
    mut suns: Query<(&mut DirectionalLight, &mut Transform), With<SunLight>>,
    mut last_hour: Local<f32>,
) {
    let hour = TimeDirector::curr_hour(&sim.sim.model.world); // 0..24
    if (*last_hour - hour).abs() < 0.01 {
        return;
    }
    *last_hour = hour;
    // Noon (12h) -> sun high; Midnight (0h/24h) -> sun below horizon
    let sun_angle = ((hour - 6.0) / 24.0) * 2.0 * PI;
    let elevation = sun_angle.sin(); // +1 at 12h, -1 at 0h
    let day_factor = ((elevation + 0.25) / 1.25).clamp(0.0, 1.0);
    let dusk_factor = (1.0 - (elevation.abs() * 3.0).clamp(0.0, 1.0)) * 0.85;

    let sky_day = Vec3::new(0.44, 0.78, 0.98);
    let sky_night = Vec3::new(0.04, 0.06, 0.14);
    let sky_dusk = Vec3::new(0.92, 0.45, 0.30);
    let mut sky = sky_night.lerp(sky_day, day_factor);
    sky = sky.lerp(sky_dusk, dusk_factor * 0.6);
    clear.0 = Color::srgb(sky.x, sky.y, sky.z);

    ambient.brightness = 140.0 + 420.0 * day_factor;
    ambient.color = if dusk_factor > 0.2 {
        Color::srgb(1.0, 0.82, 0.72)
    } else if day_factor < 0.2 {
        Color::srgb(0.45, 0.55, 0.85)
    } else {
        Color::srgb(0.95, 0.97, 1.0)
    };

    for (mut light, mut tf) in &mut suns {
        light.illuminance = 300.0 + 16_000.0 * day_factor.powf(1.2);
        let dir = Vec3::new(sun_angle.cos() * 40.0, (elevation * 45.0).max(4.0), 18.0);
        *tf = Transform::from_translation(dir).looking_at(Vec3::ZERO, Vec3::Y);
    }
}
