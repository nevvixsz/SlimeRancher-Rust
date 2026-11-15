//! Synchronization between `sr_sim::Simulation` and Bevy 3D scene entities.

use std::collections::HashMap;
use std::path::PathBuf;

use bevy::prelude::*;
use sr_ai::GordoRewardKind;
use sr_gameplay::{MarketDepositZone, WeaponVacuum};
use sr_ids::{GameMode, IdentifiableId, LandPlotId, RegionSetId};
use sr_model::LandPlotModel;
use sr_persist::GameV12;
use sr_sandbox::landmarks;
use sr_sim::{Simulation, TickEvents};
use sr_tools::WorldDump;

use crate::assets::ClientAssets;

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

#[derive(Component)]
pub struct ActorVisual {
    #[allow(dead_code)]
    pub actor_id: i64,
    #[allow(dead_code)]
    pub ident: IdentifiableId,
}

#[derive(Component)]
pub struct GordoVisual {
    #[allow(dead_code)]
    pub gordo_id: String,
}

#[derive(Component)]
pub struct VacBeamVisual;

#[derive(Component)]
pub struct VacMuzzleVisual;

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
        pos: landmarks::MARKET_CENTER + Vec3::Y * 1.2,
        radius: 2.5,
        is_market_link: false,
    });

    // Extra lively starter slimes on the arena so the 3D scene immediately pops
    for (ident, pos) in [
        (IdentifiableId::PhosphorSlime, Vec3::new(-4.0, 1.0, 7.0)),
        (IdentifiableId::HoneySlime, Vec3::new(4.5, 1.0, 8.0)),
        (IdentifiableId::RadSlime, Vec3::new(-6.0, 1.0, -3.0)),
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
                    // Place physical bodies for loaded actors when loading a save
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

    // If a WorldDump is present and no custom save replaced plots, register its plot coordinates
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
    registry.plort_mesh = meshes.add(Sphere::new(0.65).mesh().ico(1).unwrap());
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

    // Ground floor (100x100)
    let ground_mat = materials.add(StandardMaterial {
        base_color: if assets.ground_texture.is_some() {
            Color::WHITE
        } else {
            Color::srgb(0.36, 0.72, 0.34)
        },
        base_color_texture: assets.ground_texture.clone(),
        perceptual_roughness: 0.85,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(100.0, 100.0))),
        MeshMaterial3d(ground_mat),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    // Sea below the southern cliff
    let sea_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.12, 0.56, 0.86, 0.88),
        base_color_texture: assets.water_texture.clone(),
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
            Color::srgb(0.85, 0.80, 0.75)
        } else {
            Color::srgb(0.56, 0.48, 0.42)
        },
        base_color_texture: assets.wall_texture.clone(),
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

    // Corral walls around CORRAL_CENTER (semi-transparent energy fence + stone base)
    let corral_wall_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.32, 0.75, 0.95, 0.35),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });
    let cc = landmarks::CORRAL_CENTER;
    for (offset, size) in [
        (Vec3::new(0.0, 1.5, 5.25), Vec3::new(11.0, 3.0, 0.5)),
        (Vec3::new(0.0, 1.5, -5.25), Vec3::new(11.0, 3.0, 0.5)),
        (Vec3::new(5.25, 1.5, 0.0), Vec3::new(0.5, 3.0, 11.0)),
        (Vec3::new(-5.25, 1.5, 0.0), Vec3::new(0.5, 3.0, 11.0)),
    ] {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
            MeshMaterial3d(corral_wall_mat.clone()),
            Transform::from_translation(cc + offset),
        ));
    }

    // Garden soil patch
    let garden_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.45, 0.28, 0.16),
        perceptual_roughness: 0.95,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(10.0, 0.25, 10.0))),
        MeshMaterial3d(garden_mat),
        Transform::from_translation(landmarks::GARDEN_CENTER + Vec3::Y * 0.12),
    ));

    // Pond water surface
    let pond_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.18, 0.68, 0.95, 0.75),
        base_color_texture: assets.water_texture.clone(),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.15,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(8.0, 0.22, 8.0))),
        MeshMaterial3d(pond_mat),
        Transform::from_translation(landmarks::POND_CENTER + Vec3::Y * 0.11),
    ));

    // Plort Market terminal
    let market_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.18, 0.25, 0.36),
        metallic: 0.4,
        perceptual_roughness: 0.4,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(4.0, 2.5, 2.0))),
        MeshMaterial3d(market_mat),
        Transform::from_translation(landmarks::MARKET_CENTER + Vec3::Y * 1.25),
    ));

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

    // Vacpack nozzle & suction beam attached to world (updated every frame from muzzle_origin)
    let muzzle_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.25, 0.30, 0.38),
        metallic: 0.5,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(0.11, 0.55))),
        MeshMaterial3d(muzzle_mat),
        Transform::default(),
        VacMuzzleVisual,
    ));

    let beam_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.45, 0.88, 1.0, 0.30),
        emissive: LinearRgba::rgb(0.2, 0.6, 0.9),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(0.32, 6.0))),
        MeshMaterial3d(beam_mat),
        Transform::default(),
        Visibility::Hidden,
        VacBeamVisual,
    ));

    // FPS Camera
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
    assets: Res<ClientAssets>,
    sim_res: Res<SimResource>,
    mut transforms: Query<&mut Transform>,
) {
    let sim = &sim_res.sim;
    let dump = sim_res.world_dump.as_ref();

    // 1. Remove despawned / vacuumed actors
    let existing_ids: Vec<i64> = registry.actor_entities.keys().copied().collect();
    for id in existing_ids {
        if !sim.model.actors.contains_key(&id) {
            if let Some(ent) = registry.actor_entities.remove(&id) {
                commands.entity(ent).despawn_recursive();
            }
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
                    let color = ident_color(actor.ident, dump);
                    let tex = assets.item_textures.get(&actor.ident).cloned();
                    materials.add(StandardMaterial {
                        base_color: color,
                        base_color_texture: tex,
                        perceptual_roughness: if actor.ident.is_plort() { 0.15 } else { 0.45 },
                        metallic: if actor.ident.is_plort() { 0.25 } else { 0.0 },
                        ..default()
                    })
                })
                .clone();

            let mesh = if actor.ident.is_plort() {
                registry.plort_mesh.clone()
            } else if actor.ident.is_standard_crate() {
                registry.crate_mesh.clone()
            } else {
                registry.sphere_mesh.clone()
            };

            let scale = if actor.ident.is_standard_crate() {
                Vec3::splat(1.0)
            } else {
                Vec3::splat(radius)
            };

            let mut entity_cmd = commands.spawn((
                Mesh3d(mesh),
                MeshMaterial3d(mat_handle),
                Transform {
                    translation: actor.pos,
                    rotation: actor.rot,
                    scale,
                },
                ActorVisual {
                    actor_id,
                    ident: actor.ident,
                },
            ));

            // Add cute slime eyes on +Z front face
            if actor.ident.is_slime() {
                let eye_mesh = registry.eye_mesh.clone();
                let eye_mat = registry.eye_material.clone();
                let pupil_mat = registry.pupil_material.clone();
                entity_cmd.with_children(|parent| {
                    for side in [-0.32f32, 0.32f32] {
                        parent.spawn((
                            Mesh3d(eye_mesh.clone()),
                            MeshMaterial3d(eye_mat.clone()),
                            Transform::from_xyz(side, 0.20, 0.90),
                        ));
                        parent.spawn((
                            Mesh3d(eye_mesh.clone()),
                            MeshMaterial3d(pupil_mat.clone()),
                            Transform::from_xyz(side, 0.20, 0.98).with_scale(Vec3::splat(0.55)),
                        ));
                    }
                });
            }

            let ent = entity_cmd.id();
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
            let color = ident_color(gordo.slime_ident, dump);
            let mat = materials.add(StandardMaterial {
                base_color: color,
                perceptual_roughness: 0.4,
                ..default()
            });
            let eye_mesh = registry.eye_mesh.clone();
            let eye_mat = registry.eye_material.clone();
            let pupil_mat = registry.pupil_material.clone();
            let ent = commands
                .spawn((
                    Mesh3d(registry.sphere_mesh.clone()),
                    MeshMaterial3d(mat),
                    Transform::from_translation(gordo.pos + Vec3::Y * 1.9)
                        .with_scale(Vec3::splat(2.1)),
                    GordoVisual {
                        gordo_id: gordo.gordo_id.clone(),
                    },
                ))
                .with_children(|parent| {
                    for side in [-0.32f32, 0.32f32] {
                        parent.spawn((
                            Mesh3d(eye_mesh.clone()),
                            MeshMaterial3d(eye_mat.clone()),
                            Transform::from_xyz(side, 0.22, 0.90),
                        ));
                        parent.spawn((
                            Mesh3d(eye_mesh.clone()),
                            MeshMaterial3d(pupil_mat.clone()),
                            Transform::from_xyz(side, 0.22, 0.98).with_scale(Vec3::splat(0.55)),
                        ));
                    }
                })
                .id();
            registry.gordo_entities.insert(gordo.gordo_id.clone(), ent);
        }
    }

    let _ = &GordoRewardKind::SlimeKey;
}
