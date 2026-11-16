//! Detailed 3D Ranch structures (Corral, Garden, Pond, Plort Market, Gordo Dais)
//! and live synchronization of installed `LandPlotUpgrade` visuals.

use bevy::prelude::*;
use sr_ids::LandPlotUpgrade;
use sr_sandbox::landmarks;

use crate::assets::ClientAssets;
use crate::world_sync::SimResource;

#[derive(Component)]
pub struct PlotUpgradeVisual {
    pub plot_id: String,
    pub upgrade: LandPlotUpgrade,
}

#[derive(Component)]
pub struct MarketHoloPlortVisual;

#[derive(Component)]
pub struct MusicBoxCrystalVisual;

#[derive(Component)]
pub struct SprinklerHeadVisual;

#[derive(Component)]
pub struct PondFountainVisual;

pub fn spawn_ranch_structures(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    assets: &ClientAssets,
) {
    // Optional user-exported .glb/.gltf world scene from local_assets/models/
    if let Some(ref scene_handle) = assets.world_scene_model {
        commands.spawn((
            SceneRoot(scene_handle.clone()),
            Transform::IDENTITY,
        ));
    }

    let stone_mat = materials.add(StandardMaterial {
        base_color: if assets.wall_texture.is_some() {
            Color::WHITE
        } else {
            Color::srgb(0.55, 0.50, 0.46)
        },
        base_color_texture: assets.wall_texture.clone(),
        perceptual_roughness: 0.9,
        ..default()
    });
    let wood_mat = materials.add(StandardMaterial {
        base_color: if assets.wood_texture.is_some() {
            Color::WHITE
        } else {
            Color::srgb(0.54, 0.34, 0.18)
        },
        base_color_texture: assets.wood_texture.clone(),
        perceptual_roughness: 0.85,
        ..default()
    });
    let energy_wall_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.32, 0.78, 0.98, 0.30),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });
    let air_net_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.55, 0.92, 1.0, 0.20),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });
    let solar_shield_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.32, 0.14, 0.56, 0.52),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });
    let tech_metal_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.24, 0.30, 0.38),
        metallic: 0.55,
        perceptual_roughness: 0.35,
        ..default()
    });
    let cyan_glow_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.28, 0.92, 1.0, 0.82),
        emissive: LinearRgba::rgb(0.15, 0.65, 0.95),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });
    let gold_glow_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.82, 0.25),
        emissive: LinearRgba::rgb(0.35, 0.22, 0.04),
        metallic: 0.6,
        perceptual_roughness: 0.2,
        ..default()
    });

    // --- 1. CORRAL (CORRAL_CENTER) ---
    let cc = landmarks::CORRAL_CENTER;
    // 4 stone corner pillars
    let pillar_mesh = meshes.add(Cuboid::new(0.85, 3.6, 0.85));
    for (dx, dz) in [(-5.3, -5.3), (5.3, -5.3), (-5.3, 5.3), (5.3, 5.3)] {
        commands.spawn((
            Mesh3d(pillar_mesh.clone()),
            MeshMaterial3d(stone_mat.clone()),
            Transform::from_translation(cc + Vec3::new(dx, 1.8, dz)),
        ));
    }
    // Base energy walls
    for (offset, size) in [
        (Vec3::new(0.0, 1.5, 5.25), Vec3::new(10.0, 3.0, 0.22)),
        (Vec3::new(0.0, 1.5, -5.25), Vec3::new(10.0, 3.0, 0.22)),
        (Vec3::new(5.25, 1.5, 0.0), Vec3::new(0.22, 3.0, 10.0)),
        (Vec3::new(-5.25, 1.5, 0.0), Vec3::new(0.22, 3.0, 10.0)),
    ] {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
            MeshMaterial3d(energy_wall_mat.clone()),
            Transform::from_translation(cc + offset),
        ));
    }

    // Corral upgrade: High Walls
    for (offset, size) in [
        (Vec3::new(0.0, 4.1, 5.25), Vec3::new(10.0, 2.2, 0.20)),
        (Vec3::new(0.0, 4.1, -5.25), Vec3::new(10.0, 2.2, 0.20)),
        (Vec3::new(5.25, 4.1, 0.0), Vec3::new(0.20, 2.2, 10.0)),
        (Vec3::new(-5.25, 4.1, 0.0), Vec3::new(0.20, 2.2, 10.0)),
    ] {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
            MeshMaterial3d(energy_wall_mat.clone()),
            Transform::from_translation(cc + offset),
            Visibility::Hidden,
            PlotUpgradeVisual {
                plot_id: "sandbox_corral".into(),
                upgrade: LandPlotUpgrade::Walls,
            },
        ));
    }

    // Corral upgrade: AirNet canopy
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(10.6, 0.12, 10.6))),
        MeshMaterial3d(air_net_mat),
        Transform::from_translation(cc + Vec3::Y * 5.2),
        Visibility::Hidden,
        PlotUpgradeVisual {
            plot_id: "sandbox_corral".into(),
            upgrade: LandPlotUpgrade::AirNet,
        },
    ));

    // Corral upgrade: SolarShield canopy
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(11.0, 0.16, 11.0))),
        MeshMaterial3d(solar_shield_mat),
        Transform::from_translation(cc + Vec3::Y * 5.45),
        Visibility::Hidden,
        PlotUpgradeVisual {
            plot_id: "sandbox_corral".into(),
            upgrade: LandPlotUpgrade::SolarShield,
        },
    ));

    // Corral upgrade: MusicBox pedestal + crystal
    commands
        .spawn((
            Mesh3d(meshes.add(Cylinder::new(0.55, 1.8))),
            MeshMaterial3d(tech_metal_mat.clone()),
            Transform::from_translation(cc + Vec3::new(-5.9, 0.9, -4.0)),
            Visibility::Hidden,
            PlotUpgradeVisual {
                plot_id: "sandbox_corral".into(),
                upgrade: LandPlotUpgrade::MusicBox,
            },
        ))
        .with_children(|p| {
            p.spawn((
                Mesh3d(meshes.add(Sphere::new(0.42).mesh().ico(0).unwrap())),
                MeshMaterial3d(cyan_glow_mat.clone()),
                Transform::from_xyz(0.0, 1.35, 0.0),
                MusicBoxCrystalVisual,
            ));
        });

    // Corral upgrade: Auto-Feeder tower
    commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(1.1, 3.2, 1.1))),
            MeshMaterial3d(tech_metal_mat.clone()),
            Transform::from_translation(cc + Vec3::new(5.8, 1.6, 0.0)),
            Visibility::Hidden,
            PlotUpgradeVisual {
                plot_id: "sandbox_corral".into(),
                upgrade: LandPlotUpgrade::Feeder,
            },
        ))
        .with_children(|p| {
            p.spawn((
                Mesh3d(meshes.add(Cone::new(0.75, 0.9))),
                MeshMaterial3d(gold_glow_mat.clone()),
                Transform::from_xyz(-0.75, 1.1, 0.0)
                    .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
            ));
        });

    // Corral upgrade: Plort Collector tank
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(0.68, 2.4))),
        MeshMaterial3d(cyan_glow_mat.clone()),
        Transform::from_translation(cc + Vec3::new(0.0, 1.2, -5.9)),
        Visibility::Hidden,
        PlotUpgradeVisual {
            plot_id: "sandbox_corral".into(),
            upgrade: LandPlotUpgrade::PlortCollector,
        },
    ));

    // --- 2. GARDEN (GARDEN_CENTER) ---
    let gc = landmarks::GARDEN_CENTER;
    // Wooden planter rim
    for (offset, size) in [
        (Vec3::new(0.0, 0.30, 5.15), Vec3::new(10.6, 0.60, 0.35)),
        (Vec3::new(0.0, 0.30, -5.15), Vec3::new(10.6, 0.60, 0.35)),
        (Vec3::new(5.15, 0.30, 0.0), Vec3::new(0.35, 0.60, 10.6)),
        (Vec3::new(-5.15, 0.30, 0.0), Vec3::new(0.35, 0.60, 10.6)),
    ] {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
            MeshMaterial3d(wood_mat.clone()),
            Transform::from_translation(gc + offset),
        ));
    }
    // Soil bed
    let soil_mat = materials.add(StandardMaterial {
        base_color: if assets.soil_texture.is_some() {
            Color::WHITE
        } else {
            Color::srgb(0.42, 0.25, 0.14)
        },
        base_color_texture: assets.soil_texture.clone(),
        uv_transform: bevy::math::Affine2::from_scale(Vec2::splat(4.0)),
        perceptual_roughness: 0.95,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(10.0, 0.26, 10.0))),
        MeshMaterial3d(soil_mat),
        Transform::from_translation(gc + Vec3::Y * 0.13),
    ));
    // Decorative garden sprouts
    let sprout_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.24, 0.78, 0.28),
        perceptual_roughness: 0.6,
        ..default()
    });
    let sprout_mesh = meshes.add(Cone::new(0.26, 0.65));
    for gx in [-2.6_f32, 0.0, 2.6] {
        for gz in [-2.6_f32, 2.6] {
            commands.spawn((
                Mesh3d(sprout_mesh.clone()),
                MeshMaterial3d(sprout_mat.clone()),
                Transform::from_translation(gc + Vec3::new(gx, 0.55, gz)),
            ));
        }
    }
    // Garden upgrade: Sprinkler
    commands
        .spawn((
            Mesh3d(meshes.add(Cylinder::new(0.18, 1.4))),
            MeshMaterial3d(tech_metal_mat.clone()),
            Transform::from_translation(gc + Vec3::Y * 0.70),
            Visibility::Hidden,
            PlotUpgradeVisual {
                plot_id: "sandbox_garden".into(),
                upgrade: LandPlotUpgrade::Sprinkler,
            },
        ))
        .with_children(|p| {
            p.spawn((
                Mesh3d(meshes.add(Cuboid::new(1.4, 0.12, 0.14))),
                MeshMaterial3d(cyan_glow_mat.clone()),
                Transform::from_xyz(0.0, 0.72, 0.0),
                SprinklerHeadVisual,
            ));
        });
    // Garden upgrade: Scareslime post
    commands
        .spawn((
            Mesh3d(meshes.add(Cylinder::new(0.14, 2.2))),
            MeshMaterial3d(wood_mat),
            Transform::from_translation(gc + Vec3::new(-4.4, 1.1, -4.4)),
            Visibility::Hidden,
            PlotUpgradeVisual {
                plot_id: "sandbox_garden".into(),
                upgrade: LandPlotUpgrade::Scareslime,
            },
        ))
        .with_children(|p| {
            p.spawn((
                Mesh3d(meshes.add(Sphere::new(0.45).mesh().ico(2).unwrap())),
                MeshMaterial3d(gold_glow_mat.clone()),
                Transform::from_xyz(0.0, 1.1, 0.0),
            ));
        });

    // --- 3. POND (POND_CENTER) ---
    let pc = landmarks::POND_CENTER;
    for (offset, size) in [
        (Vec3::new(0.0, 0.25, 4.15), Vec3::new(8.6, 0.50, 0.35)),
        (Vec3::new(0.0, 0.25, -4.15), Vec3::new(8.6, 0.50, 0.35)),
        (Vec3::new(4.15, 0.25, 0.0), Vec3::new(0.35, 0.50, 8.6)),
        (Vec3::new(-4.15, 0.25, 0.0), Vec3::new(0.35, 0.50, 8.6)),
    ] {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
            MeshMaterial3d(stone_mat.clone()),
            Transform::from_translation(pc + offset),
        ));
    }
    let pond_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.18, 0.68, 0.95, 0.78),
        base_color_texture: assets.water_texture.clone(),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.12,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(8.0, 0.22, 8.0))),
        MeshMaterial3d(pond_mat),
        Transform::from_translation(pc + Vec3::Y * 0.11),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Torus::new(0.45, 0.65))),
        MeshMaterial3d(cyan_glow_mat.clone()),
        Transform::from_translation(pc + Vec3::Y * 0.35)
            .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
        PondFountainVisual,
    ));

    // --- 4. PLORT MARKET (MARKET_CENTER) ---
    let mc = landmarks::MARKET_CENTER;
    // Main terminal cabinet
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(4.2, 2.8, 2.0))),
        MeshMaterial3d(tech_metal_mat.clone()),
        Transform::from_translation(mc + Vec3::Y * 1.4),
    ));
    // Glowing cyan screen panel facing +Z (towards arena center)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(3.4, 1.5, 0.12))),
        MeshMaterial3d(cyan_glow_mat.clone()),
        Transform::from_translation(mc + Vec3::new(0.0, 1.9, 1.04)),
    ));
    // Deposit funnel ring in front of kiosk
    commands.spawn((
        Mesh3d(meshes.add(Torus::new(0.95, 1.25))),
        MeshMaterial3d(gold_glow_mat.clone()),
        Transform::from_translation(mc + Vec3::new(0.0, 0.85, 2.0))
            .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
    ));
    // Floating holographic plort above deposit zone
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.55).mesh().ico(0).unwrap())),
        MeshMaterial3d(cyan_glow_mat),
        Transform::from_translation(mc + Vec3::new(0.0, 1.85, 2.0))
            .with_scale(Vec3::new(0.9, 1.25, 0.9)),
        MarketHoloPlortVisual,
    ));

    // --- 5. GORDO STONE DAIS (0, 0, -14) ---
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(3.2, 0.45))),
        MeshMaterial3d(stone_mat),
        Transform::from_xyz(0.0, 0.22, -14.0),
    ));
}

pub fn sync_ranch_visuals_system(
    time: Res<Time>,
    sim_res: Res<SimResource>,
    mut upgrade_vis_q: Query<(&PlotUpgradeVisual, &mut Visibility)>,
    mut holo_q: Query<
        &mut Transform,
        (
            With<MarketHoloPlortVisual>,
            Without<MusicBoxCrystalVisual>,
            Without<SprinklerHeadVisual>,
            Without<PondFountainVisual>,
        ),
    >,
    mut music_q: Query<
        &mut Transform,
        (
            With<MusicBoxCrystalVisual>,
            Without<MarketHoloPlortVisual>,
            Without<SprinklerHeadVisual>,
            Without<PondFountainVisual>,
        ),
    >,
    mut sprinkler_q: Query<
        &mut Transform,
        (
            With<SprinklerHeadVisual>,
            Without<MarketHoloPlortVisual>,
            Without<MusicBoxCrystalVisual>,
            Without<PondFountainVisual>,
        ),
    >,
    mut pond_q: Query<
        &mut Transform,
        (
            With<PondFountainVisual>,
            Without<MarketHoloPlortVisual>,
            Without<MusicBoxCrystalVisual>,
            Without<SprinklerHeadVisual>,
        ),
    >,
) {
    let t = time.elapsed_secs();

    for (pv, mut vis) in &mut upgrade_vis_q {
        let has = sim_res
            .sim
            .model
            .land_plots
            .get(&pv.plot_id)
            .map_or(false, |plot| plot.has_upgrade(pv.upgrade));
        let target_vis = if has {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *vis != target_vis {
            *vis = target_vis;
        }
    }

    for mut tf in &mut holo_q {
        tf.rotation = Quat::from_rotation_y(t * 2.0);
        tf.translation.y = 1.85 + 0.18 * (t * 2.6).sin();
    }
    for mut tf in &mut music_q {
        tf.rotation = Quat::from_rotation_y(t * 2.4);
    }
    for mut tf in &mut sprinkler_q {
        tf.rotation = Quat::from_rotation_y(t * 4.5);
    }
    for mut tf in &mut pond_q {
        let s = 1.0 + 0.14 * (t * 2.8).sin();
        tf.scale = Vec3::new(s, s, 1.0);
    }
}
