//! Synchronization between `sr_sim::Simulation` and Bevy 3D scene entities.

use std::collections::HashMap;
use std::path::PathBuf;

use bevy::math::Affine2;
use bevy::prelude::*;
use sr_gameplay::{MarketDepositZone, WeaponVacuum};
use sr_ids::{GameMode, IdentifiableId, LandPlotId, RegionSetId};
use sr_model::LandPlotModel;
use sr_persist::GameV12;
use sr_sandbox::landmarks;
use sr_sim::{Simulation, TickEvents};
use sr_tools::WorldDump;

use crate::assets::ClientAssets;
use crate::ranch_visuals::spawn_ranch_structures;
use crate::slime_visuals::{
    init_slime_shared_assets, spawn_actor_visual, spawn_gordo_visual, SlimeSharedAssets,
};

#[derive(Resource)]
pub struct SimResource {
    pub sim: Simulation,
    pub last_events: TickEvents,
    #[allow(dead_code)]
    pub save_path: Option<PathBuf>,
    pub world_dump: Option<WorldDump>,
    pub toast: Option<(String, f32)>,
}

impl SimResource {
    pub fn set_toast(&mut self, msg: impl Into<String>) {
        self.toast = Some((msg.into(), 3.5));
    }
}

#[derive(Resource, Default)]
pub struct TargetInfo {
    pub title: String,
    pub subtitle: String,
    pub nearby_plot_id: Option<String>,
}

#[derive(Component)]
pub struct VacBeamVisual;

#[derive(Component)]
pub struct VacMuzzleVisual;

#[derive(Component)]
pub struct VacCoreVisual;

#[derive(Component)]
pub struct ShootFlashVisual;

#[derive(Component)]
pub struct AirBurstVisual;

#[derive(Resource, Default)]
pub struct VisualRegistry {
    pub actor_entities: HashMap<i64, Entity>,
    pub gordo_entities: HashMap<String, Entity>,
    pub sphere_mesh: Handle<Mesh>,
    pub plort_mesh: Handle<Mesh>,
    pub crate_mesh: Handle<Mesh>,
    pub eye_mesh: Handle<Mesh>,
    pub eye_material: Handle<StandardMaterial>,
    pub pupil_material: Handle<StandardMaterial>,
    pub vac_core_material: Handle<StandardMaterial>,
    pub materials: HashMap<IdentifiableId, Handle<StandardMaterial>>,
}

/// Canonical color for any `IdentifiableId`, preferring `WorldDump.item_colors` when available.
pub fn ident_color(id: IdentifiableId, dump: Option<&WorldDump>) -> Color {
    if let Some(d) = dump {
        if let Some(&[r, g, b, _a]) = d.item_colors.get(&id.as_i32()) {
            return Color::srgb(r, g, b);
        }
    }
    match id {
        IdentifiableId::PinkSlime | IdentifiableId::PinkPlort => Color::srgb(0.98, 0.48, 0.72),
        IdentifiableId::TabbySlime | IdentifiableId::TabbyPlort => Color::srgb(0.65, 0.67, 0.72),
        IdentifiableId::RockSlime | IdentifiableId::RockPlort => Color::srgb(0.32, 0.45, 0.78),
        IdentifiableId::PhosphorSlime | IdentifiableId::PhosphorPlort => {
            Color::srgb(0.55, 0.72, 1.0)
        }
        IdentifiableId::HoneySlime | IdentifiableId::HoneyPlort => Color::srgb(0.95, 0.72, 0.22),
        IdentifiableId::BoomSlime | IdentifiableId::BoomPlort => Color::srgb(0.90, 0.28, 0.16),
        IdentifiableId::RadSlime | IdentifiableId::RadPlort => Color::srgb(0.28, 0.92, 0.35),
        IdentifiableId::CrystalSlime | IdentifiableId::CrystalPlort => Color::srgb(0.88, 0.22, 0.56),
        IdentifiableId::HunterSlime | IdentifiableId::HunterPlort => Color::srgb(0.58, 0.40, 0.30),
        IdentifiableId::QuantumSlime | IdentifiableId::QuantumPlort => {
            Color::srgb(0.98, 0.88, 0.25)
        }
        IdentifiableId::DervishSlime | IdentifiableId::DervishPlort => {
            Color::srgb(0.68, 0.45, 0.88)
        }
        IdentifiableId::MosaicSlime | IdentifiableId::MosaicPlort => Color::srgb(0.50, 0.86, 0.88),
        IdentifiableId::TangleSlime | IdentifiableId::TanglePlort => Color::srgb(0.36, 0.78, 0.42),
        IdentifiableId::PuddleSlime | IdentifiableId::PuddlePlort => Color::srgb(0.30, 0.75, 0.95),
        IdentifiableId::FireSlime | IdentifiableId::FirePlort => Color::srgb(1.0, 0.46, 0.10),
        IdentifiableId::GoldSlime | IdentifiableId::GoldPlort => Color::srgb(1.0, 0.84, 0.12),
        IdentifiableId::LuckySlime => Color::srgb(0.88, 0.86, 0.78),
        IdentifiableId::SaberSlime | IdentifiableId::SaberPlort => Color::srgb(0.82, 0.68, 0.42),
        IdentifiableId::QuicksilverSlime | IdentifiableId::QuicksilverPlort => {
            Color::srgb(0.75, 0.82, 0.90)
        }
        IdentifiableId::GlitchSlime => Color::srgb(0.95, 0.22, 0.75),
        IdentifiableId::TarrSlime => Color::srgb(0.12, 0.10, 0.16),
        IdentifiableId::CarrotVeggie => Color::srgb(0.96, 0.50, 0.12),
        IdentifiableId::OcaocaVeggie => Color::srgb(0.76, 0.48, 0.30),
        IdentifiableId::ParsnipVeggie => Color::srgb(0.95, 0.90, 0.65),
        IdentifiableId::BeetVeggie => Color::srgb(0.74, 0.16, 0.34),
        IdentifiableId::OnionVeggie => Color::srgb(0.84, 0.72, 0.88),
        IdentifiableId::GingerVeggie => Color::srgb(0.96, 0.80, 0.35),
        IdentifiableId::PogoFruit => Color::srgb(0.98, 0.64, 0.28),
        IdentifiableId::CuberryFruit => Color::srgb(0.92, 0.28, 0.40),
        IdentifiableId::MangoFruit => Color::srgb(0.98, 0.78, 0.18),
        IdentifiableId::PearFruit => Color::srgb(0.68, 0.88, 0.28),
        IdentifiableId::LemonFruit => Color::srgb(0.98, 0.94, 0.24),
        _ if id.is_slime() => Color::srgb(0.86, 0.56, 0.82),
        _ if id.is_plort() => Color::srgb(0.70, 0.82, 0.96),
        _ if id.is_fruit() => Color::srgb(0.95, 0.55, 0.30),
        _ if id.is_veggie() => Color::srgb(0.45, 0.82, 0.35),
        _ if id.is_animal() => Color::srgb(0.95, 0.92, 0.82),
        _ if id.is_standard_crate() => Color::srgb(0.58, 0.42, 0.26),
        _ => Color::srgb(0.78, 0.78, 0.82),
    }
}

pub fn build_initial_sim(save_path: Option<PathBuf>, assets: &ClientAssets) -> SimResource {
    let world_dump = assets
        .local_root
        .as_ref()
        .map(|r| r.join("world_dump.json"))
        .filter(|p| p.is_file())
        .and_then(|p| WorldDump::load_json(&p).ok());

    let mut sim = Simulation::new(GameMode::Classic);
    sr_sandbox::setup_test_arena(&mut sim);
    sim.economy.deposit_zones.push(MarketDepositZone {
        id: "sandbox_market_zone".to_string(),
        pos: landmarks::MARKET_CENTER + Vec3::new(0.0, 1.1, 2.0),
        radius: 2.6,
        is_market_link: false,
    });

    // Extra lively starter slimes & food on the arena so the 3D scene immediately pops
    for (ident, pos) in [
        (IdentifiableId::PhosphorSlime, Vec3::new(-4.0, 1.0, 7.0)),
        (IdentifiableId::HoneySlime, Vec3::new(4.5, 1.0, 8.0)),
        (IdentifiableId::RadSlime, Vec3::new(-6.0, 1.0, -3.0)),
        (IdentifiableId::CrystalSlime, landmarks::CORRAL_CENTER + Vec3::new(2.0, 1.0, -1.5)),
        (IdentifiableId::PogoFruit, landmarks::GARDEN_CENTER + Vec3::new(-1.0, 0.5, -1.0)),
        (IdentifiableId::Hen, Vec3::new(3.0, 0.8, -5.0)),
    ] {
        sim.spawn_actor(ident, RegionSetId::Home, pos, Vec3::ZERO);
    }

    let mut loaded_msg = None;
    if let Some(ref sp) = save_path {
        if sp.is_file() {
            match std::fs::read(sp)
                .map_err(sr_persist::PersistError::from)
                .and_then(|b| GameV12::from_bytes(&b))
                .and_then(|v12| v12.into_game_model())
            {
                Ok(model) => {
                    sim.model = model;
                    for actor in sim.model.actors.values() {
                        let radius = WeaponVacuum::radius_of(actor.ident);
                        sim.physics.insert_body(sr_physics::RigidBody {
                            actor_id: actor.actor_id,
                            pos: actor.pos,
                            radius,
                            ..Default::default()
                        });
                    }
                    loaded_msg = Some(format!("Loaded save: {}", sp.display()));
                }
                Err(err) => {
                    warn!("Failed to load save {}: {}", sp.display(), err);
                    loaded_msg = Some(format!("Save load error: {err}"));
                }
            }
        }
    }

    if let Some(ref dump) = world_dump {
        for p in &dump.land_plots {
            sim.model
                .land_plots
                .entry(p.id.clone())
                .or_insert_with(|| LandPlotModel::new(&p.id, LandPlotId::Empty));
        }
    }

    let mut res = SimResource {
        sim,
        last_events: TickEvents::default(),
        save_path,
        world_dump,
        toast: None,
    };
    if let Some(msg) = loaded_msg {
        res.set_toast(msg);
    }
    res
}

pub fn setup_world_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut registry: ResMut<VisualRegistry>,
    assets: Res<ClientAssets>,
    sim_res: Res<SimResource>,
) {
    registry.sphere_mesh = meshes.add(Sphere::new(1.0).mesh().ico(4).unwrap());
    registry.plort_mesh = meshes.add(Sphere::new(0.65).mesh().ico(0).unwrap());
    registry.crate_mesh = meshes.add(Cuboid::new(1.3, 1.3, 1.3));
    registry.eye_mesh = meshes.add(Sphere::new(0.14).mesh().uv(12, 8));
    registry.eye_material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        unlit: true,
        ..default()
    });
    registry.pupil_material = materials.add(StandardMaterial {
        base_color: Color::BLACK,
        unlit: true,
        ..default()
    });
    registry.vac_core_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.32, 0.85, 1.0),
        emissive: LinearRgba::rgb(0.18, 0.55, 0.85),
        perceptual_roughness: 0.25,
        ..default()
    });

    let shared = init_slime_shared_assets(&mut meshes, &mut materials);
    commands.insert_resource(shared);

    // Ground floor (100x100) with crisp tiled UV repeat
    let ground_mat = materials.add(StandardMaterial {
        base_color: if assets.ground_texture.is_some() {
            Color::WHITE
        } else {
            Color::srgb(0.36, 0.72, 0.34)
        },
        base_color_texture: assets.ground_texture.clone(),
        uv_transform: Affine2::from_scale(Vec2::splat(25.0)),
        perceptual_roughness: 0.85,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(100.0, 100.0))),
        MeshMaterial3d(ground_mat.clone()),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    // Optional raw .obj world mesh from local_assets/Mesh/
    if assets.world_scene_model.is_none() {
        if let Some(ref world_obj) = assets.world_obj_mesh {
            commands.spawn((
                Mesh3d(world_obj.clone()),
                MeshMaterial3d(ground_mat),
                Transform::IDENTITY,
            ));
        }
    }

    // Sea below the southern cliff
    let sea_mat = materials.add(StandardMaterial {
        base_color: if assets.water_texture.is_some() {
            Color::srgba(0.65, 0.88, 1.0, 0.88)
        } else {
            Color::srgba(0.12, 0.56, 0.86, 0.88)
        },
        base_color_texture: assets.water_texture.clone(),
        uv_transform: Affine2::from_scale(Vec2::splat(60.0)),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.2,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(300.0, 300.0))),
        MeshMaterial3d(sea_mat),
        Transform::from_xyz(0.0, -8.0, 0.0),
    ));

    // Perimeter low walls
    let wall_mat = materials.add(StandardMaterial {
        base_color: if assets.wall_texture.is_some() {
            Color::WHITE
        } else {
            Color::srgb(0.56, 0.48, 0.42)
        },
        base_color_texture: assets.wall_texture.clone(),
        uv_transform: Affine2::from_scale(Vec2::new(20.0, 1.0)),
        perceptual_roughness: 0.9,
        ..default()
    });
    for (pos, size) in [
        (Vec3::new(0.0, 2.0, 50.0), Vec3::new(100.0, 4.0, 1.0)),
        (Vec3::new(50.0, 2.0, 0.0), Vec3::new(1.0, 4.0, 100.0)),
        (Vec3::new(-50.0, 2.0, 0.0), Vec3::new(1.0, 4.0, 100.0)),
    ] {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
            MeshMaterial3d(wall_mat.clone()),
            Transform::from_translation(pos),
        ));
    }

    // Ranch structures & upgrades (Corral, Garden, Pond, Plort Market, Gordo Dais, World GLTF)
    spawn_ranch_structures(&mut commands, &mut meshes, &mut materials, &assets);

    // WorldDump markers (if user dumped real scene coordinates)
    if let Some(ref dump) = sim_res.world_dump {
        let pod_mat = materials.add(StandardMaterial {
            base_color: Color::srgb(0.25, 0.92, 0.72),
            emissive: LinearRgba::rgb(0.05, 0.3, 0.2),
            ..default()
        });
        for pod in &dump.treasure_pods {
            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.8, 0.8, 0.8))),
                MeshMaterial3d(pod_mat.clone()),
                Transform::from_xyz(pod.pos.x, pod.pos.y + 0.4, pod.pos.z),
            ));
        }
    }

    // Multi-part Vacpack viewmodel (original `.obj` if available, else procedural barrel + intake ring + core)
    let muzzle_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.24, 0.30, 0.38),
        metallic: 0.55,
        perceptual_roughness: 0.35,
        ..default()
    });
    let ring_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.92, 0.42, 0.16),
        metallic: 0.4,
        perceptual_roughness: 0.3,
        ..default()
    });
    let vac_barrel_mesh = assets
        .vacpack_mesh
        .clone()
        .unwrap_or_else(|| meshes.add(Cylinder::new(0.11, 0.60)));
    let has_orig_vac = assets.vacpack_mesh.is_some();
    commands
        .spawn((
            Mesh3d(vac_barrel_mesh),
            MeshMaterial3d(muzzle_mat),
            Transform::default(),
            VacMuzzleVisual,
        ))
        .with_children(|p| {
            if !has_orig_vac {
                // Intake ring at front tip (+Y in local cylinder space)
                p.spawn((
                    Mesh3d(meshes.add(Torus::new(0.09, 0.15))),
                    MeshMaterial3d(ring_mat),
                    Transform::from_xyz(0.0, 0.30, 0.0)
                        .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                ));
            }
            // Glowing slot-color tank core
            p.spawn((
                Mesh3d(meshes.add(Cylinder::new(0.125, 0.20))),
                MeshMaterial3d(registry.vac_core_material.clone()),
                Transform::from_xyz(0.0, -0.05, 0.0),
                VacCoreVisual,
            ));
        });

    // Animated vacuum cone beam
    let beam_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.45, 0.88, 1.0, 0.28),
        emissive: LinearRgba::rgb(0.20, 0.65, 0.95),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Cone::new(0.52, 6.2))),
        MeshMaterial3d(beam_mat),
        Transform::default(),
        Visibility::Hidden,
        VacBeamVisual,
    ));

    // Brief shoot muzzle flash
    let flash_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.78, 0.28, 0.75),
        emissive: LinearRgba::rgb(0.95, 0.55, 0.10),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.22).mesh().ico(1).unwrap())),
        MeshMaterial3d(flash_mat),
        Transform::default(),
        Visibility::Hidden,
        ShootFlashVisual,
    ));

    // Expanding AirBurst shockwave torus
    let burst_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.65, 0.95, 1.0, 0.55),
        emissive: LinearRgba::rgb(0.35, 0.85, 1.0),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Torus::new(0.82, 1.0))),
        MeshMaterial3d(burst_mat),
        Transform::default(),
        Visibility::Hidden,
        AirBurstVisual,
    ));

    // FPS Camera (Msaa::Off default for DX12 FL 11_0 compatibility; opt-in via SR_MSAA=1)
    let msaa = if std::env::var_os("SR_MSAA").is_some() {
        Msaa::Sample4
    } else {
        Msaa::Off
    };
    commands.spawn((
        Camera3d::default(),
        msaa,
        Projection::Perspective(PerspectiveProjection {
            fov: 72.0f32.to_radians(),
            ..default()
        }),
        Transform::from_xyz(0.0, 1.55, 0.0).looking_to(Vec3::Z, Vec3::Y),
    ));
}

pub fn sync_actors_and_gordos_system(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut registry: ResMut<VisualRegistry>,
    shared: Res<SlimeSharedAssets>,
    assets: Res<ClientAssets>,
    sim_res: Res<SimResource>,
    mut transforms: Query<&mut Transform>,
    mut scratch_despawn: Local<Vec<i64>>,
) {
    let sim = &sim_res.sim;
    let dump = sim_res.world_dump.as_ref();

    // 1. Remove despawned / vacuumed actors (reusing Local scratch buffer = zero allocations)
    scratch_despawn.clear();
    for &id in registry.actor_entities.keys() {
        if !sim.model.actors.contains_key(&id) {
            scratch_despawn.push(id);
        }
    }
    for id in scratch_despawn.iter().copied() {
        if let Some(ent) = registry.actor_entities.remove(&id) {
            commands.entity(ent).despawn_recursive();
        }
    }

    // 2. Spawn or update active actors
    for (&actor_id, actor) in &sim.model.actors {
        let radius = WeaponVacuum::radius_of(actor.ident);
        if let Some(&ent) = registry.actor_entities.get(&actor_id) {
            if let Ok(mut tf) = transforms.get_mut(ent) {
                tf.translation = actor.pos;
                tf.rotation = actor.rot;
            }
        } else {
            let mat_handle = registry
                .materials
                .entry(actor.ident)
                .or_insert_with(|| {
                    let tex = assets.item_textures.get(&actor.ident).cloned();
                    let color = if tex.is_some() {
                        Color::WHITE
                    } else {
                        ident_color(actor.ident, dump)
                    };
                    materials.add(StandardMaterial {
                        base_color: color,
                        base_color_texture: tex,
                        perceptual_roughness: if actor.ident.is_plort() { 0.15 } else { 0.45 },
                        metallic: if actor.ident.is_plort() { 0.30 } else { 0.0 },
                        ..default()
                    })
                })
                .clone();

            let ent = spawn_actor_visual(
                &mut commands,
                &registry,
                &shared,
                &assets,
                actor_id,
                actor.ident,
                actor.pos,
                actor.rot,
                radius,
                mat_handle,
            );
            registry.actor_entities.insert(actor_id, ent);
        }
    }

    // 3. Sync world Gordos (static giant slimes until popped)
    for gordo in &sim.ai.world_gordos {
        let popped = sim
            .model
            .gordos
            .get(&gordo.gordo_id)
            .map_or(false, |gm| gm.gordo_eaten_count == -1);
        if popped {
            if let Some(ent) = registry.gordo_entities.remove(&gordo.gordo_id) {
                commands.entity(ent).despawn_recursive();
            }
            continue;
        }
        if !registry.gordo_entities.contains_key(&gordo.gordo_id) {
            let mat = registry
                .materials
                .entry(gordo.slime_ident)
                .or_insert_with(|| {
                    let tex = assets.item_textures.get(&gordo.slime_ident).cloned();
                    let color = if tex.is_some() {
                        Color::WHITE
                    } else {
                        ident_color(gordo.slime_ident, dump)
                    };
                    materials.add(StandardMaterial {
                        base_color: color,
                        base_color_texture: tex,
                        perceptual_roughness: 0.4,
                        ..default()
                    })
                })
                .clone();
            let ent = spawn_gordo_visual(
                &mut commands,
                &registry,
                &shared,
                &assets,
                gordo.gordo_id.clone(),
                gordo.slime_ident,
                gordo.pos,
                mat,
            );
            registry.gordo_entities.insert(gordo.gordo_id.clone(), ent);
        }
    }
}
