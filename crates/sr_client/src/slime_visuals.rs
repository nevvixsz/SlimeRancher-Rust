//! Procedural species silhouettes, expressive slime faces, squash-and-stretch animation,
//! and distinct food/plort/crate/gordo visuals.

use bevy::prelude::*;
use sr_ids::IdentifiableId;
use sr_model::ActorKind;

use crate::assets::ClientAssets;
use crate::world_sync::{SimResource, VisualRegistry};

#[derive(Component)]
pub struct SlimeBodyVisual {
    pub actor_id: i64,
    pub base_scale: f32,
    pub is_puddle: bool,
}

#[derive(Component)]
pub struct SlimeMouthVisual {
    pub actor_id: i64,
}

#[derive(Component)]
pub struct SlimeBrowsVisual {
    pub actor_id: i64,
}

#[derive(Component)]
pub struct SlimeAuraVisual {
    pub actor_id: i64,
    pub is_tarr: bool,
}

#[derive(Component)]
pub struct GordoBodyVisual {
    #[allow(dead_code)]
    pub gordo_id: String,
    pub base_pos: Vec3,
}

#[derive(Component)]
pub struct GordoKeyVisual;

#[derive(Resource, Default)]
pub struct SlimeSharedAssets {
    pub carrot_mesh: Handle<Mesh>,
    pub leaf_mesh: Handle<Mesh>,
    pub ear_mesh: Handle<Mesh>,
    pub spike_mesh: Handle<Mesh>,
    pub wing_mesh: Handle<Mesh>,
    pub halo_mesh: Handle<Mesh>,
    pub mouth_mesh: Handle<Mesh>,
    pub brow_mesh: Handle<Mesh>,
    pub aura_mesh: Handle<Mesh>,
    pub beak_mesh: Handle<Mesh>,
    pub key_shaft_mesh: Handle<Mesh>,

    pub mouth_dark_material: Handle<StandardMaterial>,
    pub mouth_angry_material: Handle<StandardMaterial>,
    pub leaf_material: Handle<StandardMaterial>,
    pub beak_material: Handle<StandardMaterial>,
    pub comb_material: Handle<StandardMaterial>,
    pub rock_spike_material: Handle<StandardMaterial>,
    pub crystal_spike_material: Handle<StandardMaterial>,
    pub phosphor_wing_material: Handle<StandardMaterial>,
    pub rad_aura_material: Handle<StandardMaterial>,
    pub tarr_aura_material: Handle<StandardMaterial>,
    pub honey_cap_material: Handle<StandardMaterial>,
    pub fire_crest_material: Handle<StandardMaterial>,
    pub gold_key_material: Handle<StandardMaterial>,
}

pub fn init_slime_shared_assets(
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) -> SlimeSharedAssets {
    SlimeSharedAssets {
        carrot_mesh: meshes.add(Cone::new(0.32, 1.15)),
        leaf_mesh: meshes.add(Cone::new(0.20, 0.45)),
        ear_mesh: meshes.add(Cone::new(0.24, 0.55)),
        spike_mesh: meshes.add(Cone::new(0.22, 0.62)),
        wing_mesh: meshes.add(Cuboid::new(0.65, 0.08, 0.34)),
        halo_mesh: meshes.add(Torus::new(0.26, 0.36)),
        mouth_mesh: meshes.add(Sphere::new(0.14).mesh().uv(10, 6)),
        brow_mesh: meshes.add(Cuboid::new(0.24, 0.06, 0.08)),
        aura_mesh: meshes.add(Sphere::new(1.55).mesh().ico(2).unwrap()),
        beak_mesh: meshes.add(Cone::new(0.16, 0.36)),
        key_shaft_mesh: meshes.add(Cylinder::new(0.08, 0.65)),

        mouth_dark_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.18, 0.06, 0.10),
            unlit: true,
            ..default()
        }),
        mouth_angry_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.95, 0.18, 0.22),
            emissive: LinearRgba::rgb(0.35, 0.04, 0.06),
            unlit: true,
            ..default()
        }),
        leaf_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.22, 0.78, 0.28),
            perceptual_roughness: 0.6,
            ..default()
        }),
        beak_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.98, 0.68, 0.16),
            perceptual_roughness: 0.45,
            ..default()
        }),
        comb_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.92, 0.20, 0.22),
            perceptual_roughness: 0.5,
            ..default()
        }),
        rock_spike_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.28, 0.48, 0.88),
            metallic: 0.25,
            perceptual_roughness: 0.35,
            ..default()
        }),
        crystal_spike_material: materials.add(StandardMaterial {
            base_color: Color::srgba(0.95, 0.30, 0.70, 0.90),
            emissive: LinearRgba::rgb(0.25, 0.05, 0.18),
            alpha_mode: AlphaMode::Blend,
            perceptual_roughness: 0.15,
            ..default()
        }),
        phosphor_wing_material: materials.add(StandardMaterial {
            base_color: Color::srgba(0.75, 0.90, 1.0, 0.55),
            emissive: LinearRgba::rgb(0.25, 0.45, 0.85),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        }),
        rad_aura_material: materials.add(StandardMaterial {
            base_color: Color::srgba(0.25, 0.95, 0.35, 0.20),
            emissive: LinearRgba::rgb(0.08, 0.45, 0.12),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        }),
        tarr_aura_material: materials.add(StandardMaterial {
            base_color: Color::srgba(0.70, 0.15, 0.85, 0.22),
            emissive: LinearRgba::rgb(0.40, 0.08, 0.50),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        }),
        honey_cap_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.98, 0.72, 0.12),
            perceptual_roughness: 0.20,
            ..default()
        }),
        fire_crest_material: materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.55, 0.10),
            emissive: LinearRgba::rgb(0.75, 0.28, 0.04),
            perceptual_roughness: 0.3,
            ..default()
        }),
        gold_key_material: materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.86, 0.20),
            emissive: LinearRgba::rgb(0.35, 0.25, 0.02),
            metallic: 0.7,
            perceptual_roughness: 0.2,
            ..default()
        }),
    }
}

fn spawn_slime_species_traits(
    parent: &mut ChildBuilder,
    ident: IdentifiableId,
    shared: &SlimeSharedAssets,
    body_mat: Handle<StandardMaterial>,
) {
    match ident {
        IdentifiableId::TabbySlime
        | IdentifiableId::HunterSlime
        | IdentifiableId::LuckySlime => {
            for side in [-0.52_f32, 0.52_f32] {
                let tilt = if side < 0.0 { 0.35 } else { -0.35 };
                parent.spawn((
                    Mesh3d(shared.ear_mesh.clone()),
                    MeshMaterial3d(body_mat.clone()),
                    Transform::from_xyz(side, 0.88, 0.08)
                        .with_rotation(Quat::from_rotation_z(tilt)),
                ));
            }
        }
        IdentifiableId::RockSlime => {
            for (x, y, z, rx, rz) in [
                (0.0_f32, 1.02_f32, 0.0_f32, 0.0_f32, 0.0_f32),
                (-0.48, 0.82, -0.15, -0.25, 0.45),
                (0.48, 0.82, -0.15, -0.25, -0.45),
                (0.0, 0.68, -0.65, -0.75, 0.0),
            ] {
                parent.spawn((
                    Mesh3d(shared.spike_mesh.clone()),
                    MeshMaterial3d(shared.rock_spike_material.clone()),
                    Transform::from_xyz(x, y, z)
                        .with_rotation(Quat::from_euler(EulerRot::XYZ, rx, 0.0, rz)),
                ));
            }
        }
        IdentifiableId::CrystalSlime => {
            for (x, y, z, rx, rz) in [
                (0.0_f32, 1.05_f32, 0.0_f32, 0.0_f32, 0.0_f32),
                (-0.52, 0.85, 0.0, 0.0, 0.55),
                (0.52, 0.85, 0.0, 0.0, -0.55),
                (0.0, 0.72, -0.62, -0.70, 0.0),
            ] {
                parent.spawn((
                    Mesh3d(shared.spike_mesh.clone()),
                    MeshMaterial3d(shared.crystal_spike_material.clone()),
                    Transform::from_xyz(x, y, z)
                        .with_rotation(Quat::from_euler(EulerRot::XYZ, rx, 0.0, rz)),
                ));
            }
        }
        IdentifiableId::PhosphorSlime => {
            for side in [-1.0_f32, 1.0_f32] {
                parent.spawn((
                    Mesh3d(shared.wing_mesh.clone()),
                    MeshMaterial3d(shared.phosphor_wing_material.clone()),
                    Transform::from_xyz(side * 0.92, 0.35, -0.20)
                        .with_rotation(Quat::from_rotation_z(side * 0.35)),
                ));
            }
            parent.spawn((
                Mesh3d(shared.halo_mesh.clone()),
                MeshMaterial3d(shared.phosphor_wing_material.clone()),
                Transform::from_xyz(0.0, 1.25, 0.0),
            ));
        }
        IdentifiableId::HoneySlime => {
            parent.spawn((
                Mesh3d(shared.leaf_mesh.clone()),
                MeshMaterial3d(shared.honey_cap_material.clone()),
                Transform::from_xyz(0.0, 0.95, -0.05)
                    .with_scale(Vec3::new(1.8, 0.65, 1.8)),
            ));
        }
        IdentifiableId::FireSlime => {
            for (x, z) in [(0.0_f32, 0.0_f32), (-0.30, -0.15), (0.30, -0.15)] {
                parent.spawn((
                    Mesh3d(shared.spike_mesh.clone()),
                    MeshMaterial3d(shared.fire_crest_material.clone()),
                    Transform::from_xyz(x, 0.92, z).with_scale(Vec3::splat(0.75)),
                ));
            }
        }
        _ => {}
    }
}

pub fn spawn_actor_visual(
    commands: &mut Commands,
    registry: &VisualRegistry,
    shared: &SlimeSharedAssets,
    assets: &ClientAssets,
    actor_id: i64,
    ident: IdentifiableId,
    pos: Vec3,
    rot: Quat,
    radius: f32,
    mat_handle: Handle<StandardMaterial>,
) -> Entity {
    let is_puddle = ident == IdentifiableId::PuddleSlime;
    let species_mesh = assets.item_meshes.get(&ident).cloned();

    if ident.is_slime() {
        let has_species_mesh = species_mesh.is_some();
        let body_mesh = species_mesh
            .or_else(|| assets.slime_base_mesh.clone())
            .unwrap_or_else(|| registry.sphere_mesh.clone());
        let scale = if is_puddle {
            Vec3::new(radius * 1.15, radius * 0.65, radius * 1.15)
        } else {
            Vec3::splat(radius)
        };
        return commands
            .spawn((
                Mesh3d(body_mesh),
                MeshMaterial3d(mat_handle.clone()),
                Transform {
                    translation: pos,
                    rotation: rot,
                    scale,
                },
                SlimeBodyVisual {
                    actor_id,
                    base_scale: radius,
                    is_puddle,
                },
            ))
            .with_children(|parent| {
                // Eyes + pupils + dynamic mouth/brows (always attached unless species mesh replaces full rig)
                if !has_species_mesh {
                    for side in [-0.32_f32, 0.32_f32] {
                        parent.spawn((
                            Mesh3d(registry.eye_mesh.clone()),
                            MeshMaterial3d(registry.eye_material.clone()),
                            Transform::from_xyz(side, 0.22, 0.90),
                        ));
                        parent.spawn((
                            Mesh3d(registry.eye_mesh.clone()),
                            MeshMaterial3d(registry.pupil_material.clone()),
                            Transform::from_xyz(side, 0.22, 0.98).with_scale(Vec3::splat(0.55)),
                        ));
                    }

                    // Dynamic mouth
                    parent.spawn((
                        Mesh3d(shared.mouth_mesh.clone()),
                        MeshMaterial3d(shared.mouth_dark_material.clone()),
                        Transform::from_xyz(0.0, -0.10, 0.95)
                            .with_scale(Vec3::new(1.3, 0.55, 0.4)),
                        SlimeMouthVisual { actor_id },
                    ));

                    // Dynamic angry brows (hidden unless agitated / feral / tarr)
                    parent
                        .spawn((
                            Transform::from_xyz(0.0, 0.42, 0.92),
                            Visibility::Hidden,
                            SlimeBrowsVisual { actor_id },
                        ))
                        .with_children(|brows| {
                            for side in [-0.32_f32, 0.32_f32] {
                                let tilt = if side < 0.0 { -0.35 } else { 0.35 };
                                brows.spawn((
                                    Mesh3d(shared.brow_mesh.clone()),
                                    MeshMaterial3d(shared.mouth_angry_material.clone()),
                                    Transform::from_xyz(side, 0.0, 0.0)
                                        .with_rotation(Quat::from_rotation_z(tilt)),
                                ));
                            }
                        });

                    // Species traits
                    spawn_slime_species_traits(parent, ident, shared, mat_handle.clone());
                }

                // Rad / Tarr aura
                if ident == IdentifiableId::RadSlime || ident == IdentifiableId::TarrSlime {
                    let is_tarr = ident == IdentifiableId::TarrSlime;
                    let aura_mat = if is_tarr {
                        shared.tarr_aura_material.clone()
                    } else {
                        shared.rad_aura_material.clone()
                    };
                    parent.spawn((
                        Mesh3d(shared.aura_mesh.clone()),
                        MeshMaterial3d(aura_mat),
                        Transform::IDENTITY,
                        SlimeAuraVisual { actor_id, is_tarr },
                    ));
                }
            })
            .id();
    }

    if ident.is_plort() {
        let plort_mesh = species_mesh
            .or_else(|| assets.plort_base_mesh.clone())
            .unwrap_or_else(|| registry.plort_mesh.clone());
        return commands
            .spawn((
                Mesh3d(plort_mesh),
                MeshMaterial3d(mat_handle),
                Transform {
                    translation: pos,
                    rotation: rot,
                    scale: Vec3::new(radius * 0.95, radius * 1.25, radius * 0.95),
                },
            ))
            .id();
    }

    if let Some(orig_mesh) = species_mesh {
        return commands
            .spawn((
                Mesh3d(orig_mesh),
                MeshMaterial3d(mat_handle),
                Transform {
                    translation: pos,
                    rotation: rot,
                    scale: Vec3::splat(radius),
                },
            ))
            .id();
    }

    if ident.is_veggie() {
        let is_carrot = ident == IdentifiableId::CarrotVeggie
            || ident == IdentifiableId::ParsnipVeggie
            || ident == IdentifiableId::GingerVeggie;
        let body_mesh = if is_carrot {
            shared.carrot_mesh.clone()
        } else {
            registry.sphere_mesh.clone()
        };
        let body_scale = if is_carrot {
            Vec3::splat(radius * 1.1)
        } else {
            Vec3::new(radius, radius * 0.88, radius)
        };
        return commands
            .spawn((
                Mesh3d(body_mesh),
                MeshMaterial3d(mat_handle),
                Transform {
                    translation: pos,
                    rotation: if is_carrot {
                        rot * Quat::from_rotation_x(std::f32::consts::PI)
                    } else {
                        rot
                    },
                    scale: body_scale,
                },
            ))
            .with_children(|parent| {
                let leaf_y = if is_carrot { -0.55 } else { 0.95 };
                let leaf_rot = if is_carrot {
                    Quat::from_rotation_x(std::f32::consts::PI)
                } else {
                    Quat::IDENTITY
                };
                parent.spawn((
                    Mesh3d(shared.leaf_mesh.clone()),
                    MeshMaterial3d(shared.leaf_material.clone()),
                    Transform::from_xyz(0.0, leaf_y, 0.0).with_rotation(leaf_rot),
                ));
            })
            .id();
    }

    if ident.is_fruit() {
        return commands
            .spawn((
                Mesh3d(registry.sphere_mesh.clone()),
                MeshMaterial3d(mat_handle),
                Transform {
                    translation: pos,
                    rotation: rot,
                    scale: Vec3::splat(radius * 0.95),
                },
            ))
            .with_children(|parent| {
                parent.spawn((
                    Mesh3d(shared.leaf_mesh.clone()),
                    MeshMaterial3d(shared.leaf_material.clone()),
                    Transform::from_xyz(0.12, 0.92, 0.0)
                        .with_rotation(Quat::from_rotation_z(-0.45))
                        .with_scale(Vec3::splat(0.75)),
                ));
            })
            .id();
    }

    if ident.is_animal() {
        return commands
            .spawn((
                Mesh3d(registry.sphere_mesh.clone()),
                MeshMaterial3d(mat_handle),
                Transform {
                    translation: pos,
                    rotation: rot,
                    scale: Vec3::new(radius * 0.95, radius * 0.88, radius * 1.15),
                },
            ))
            .with_children(|parent| {
                // Beak
                parent.spawn((
                    Mesh3d(shared.beak_mesh.clone()),
                    MeshMaterial3d(shared.beak_material.clone()),
                    Transform::from_xyz(0.0, 0.10, 1.05)
                        .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                ));
                // Red comb
                parent.spawn((
                    Mesh3d(shared.spike_mesh.clone()),
                    MeshMaterial3d(shared.comb_material.clone()),
                    Transform::from_xyz(0.0, 0.95, 0.25).with_scale(Vec3::splat(0.55)),
                ));
            })
            .id();
    }

    if ident.is_standard_crate() {
        return commands
            .spawn((
                Mesh3d(registry.crate_mesh.clone()),
                MeshMaterial3d(mat_handle),
                Transform {
                    translation: pos,
                    rotation: rot,
                    scale: Vec3::ONE,
                },
            ))
            .id();
    }

    // Fallback generic object
    commands
        .spawn((
            Mesh3d(registry.sphere_mesh.clone()),
            MeshMaterial3d(mat_handle),
            Transform {
                translation: pos,
                rotation: rot,
                scale: Vec3::splat(radius),
            },
        ))
        .id()
}

pub fn spawn_gordo_visual(
    commands: &mut Commands,
    registry: &VisualRegistry,
    shared: &SlimeSharedAssets,
    assets: &ClientAssets,
    gordo_id: String,
    slime_ident: IdentifiableId,
    pos: Vec3,
    mat: Handle<StandardMaterial>,
) -> Entity {
    let base_pos = pos + Vec3::Y * 2.05;
    let species_mesh = assets.item_meshes.get(&slime_ident).cloned();
    let has_species_mesh = species_mesh.is_some();
    let body_mesh = species_mesh
        .or_else(|| assets.slime_base_mesh.clone())
        .unwrap_or_else(|| registry.sphere_mesh.clone());
    commands
        .spawn((
            Mesh3d(body_mesh),
            MeshMaterial3d(mat.clone()),
            Transform::from_translation(base_pos).with_scale(Vec3::splat(2.15)),
            GordoBodyVisual { gordo_id, base_pos },
        ))
        .with_children(|parent| {
            if !has_species_mesh {
                for side in [-0.32_f32, 0.32_f32] {
                    parent.spawn((
                        Mesh3d(registry.eye_mesh.clone()),
                        MeshMaterial3d(registry.eye_material.clone()),
                        Transform::from_xyz(side, 0.22, 0.90),
                    ));
                    parent.spawn((
                        Mesh3d(registry.eye_mesh.clone()),
                        MeshMaterial3d(registry.pupil_material.clone()),
                        Transform::from_xyz(side, 0.22, 0.98).with_scale(Vec3::splat(0.55)),
                    ));
                }
                parent.spawn((
                    Mesh3d(shared.mouth_mesh.clone()),
                    MeshMaterial3d(shared.mouth_dark_material.clone()),
                    Transform::from_xyz(0.0, -0.10, 0.94)
                        .with_scale(Vec3::new(1.6, 0.75, 0.45)),
                ));
                spawn_slime_species_traits(parent, slime_ident, shared, mat);
            }

            // Floating SlimeKey indicator above the Gordo
            parent
                .spawn((
                    Transform::from_xyz(0.0, 1.45, 0.0),
                    Visibility::Inherited,
                    GordoKeyVisual,
                ))
                .with_children(|key| {
                    key.spawn((
                        Mesh3d(shared.halo_mesh.clone()),
                        MeshMaterial3d(shared.gold_key_material.clone()),
                        Transform::from_xyz(0.0, 0.22, 0.0).with_scale(Vec3::splat(0.65)),
                    ));
                    key.spawn((
                        Mesh3d(shared.key_shaft_mesh.clone()),
                        MeshMaterial3d(shared.gold_key_material.clone()),
                        Transform::from_xyz(0.0, -0.12, 0.0).with_scale(Vec3::splat(0.75)),
                    ));
                });
        })
        .id()
}

pub fn update_slime_animations_system(
    time: Res<Time>,
    sim_res: Res<SimResource>,
    mut bodies: Query<(&SlimeBodyVisual, &mut Transform), Without<GordoBodyVisual>>,
    mut mouths: Query<
        (&SlimeMouthVisual, &mut Transform),
        (Without<SlimeBodyVisual>, Without<GordoBodyVisual>),
    >,
    mut brows: Query<(&SlimeBrowsVisual, &mut Visibility)>,
    mut auras: Query<
        (&SlimeAuraVisual, &mut Transform),
        (
            Without<SlimeBodyVisual>,
            Without<SlimeMouthVisual>,
            Without<GordoBodyVisual>,
        ),
    >,
    mut gordos: Query<(&GordoBodyVisual, &mut Transform), Without<SlimeBodyVisual>>,
    mut keys: Query<
        &mut Transform,
        (
            With<GordoKeyVisual>,
            Without<SlimeBodyVisual>,
            Without<SlimeMouthVisual>,
            Without<SlimeAuraVisual>,
            Without<GordoBodyVisual>,
        ),
    >,
) {
    let t = time.elapsed_secs();
    let sim = &sim_res.sim;

    // 1. Squash & stretch on slime bodies from vertical velocity + subtle idle jiggle
    for (sb, mut tf) in &mut bodies {
        let vy = sim
            .physics
            .bodies
            .get(&sb.actor_id)
            .map(|b| b.vel.y)
            .unwrap_or(0.0);
        let stretch = (vy * 0.035).clamp(-0.22, 0.28);
        let idle = (t * 4.5 + sb.actor_id as f32 * 0.7).sin() * 0.035;
        let y_factor = (if sb.is_puddle { 0.65 } else { 1.0 }) + stretch + idle;
        let xz_factor = (if sb.is_puddle { 1.15 } else { 1.0 }) - (stretch + idle) * 0.45;
        tf.scale = Vec3::new(
            sb.base_scale * xz_factor.max(0.65),
            sb.base_scale * y_factor.max(0.50),
            sb.base_scale * xz_factor.max(0.65),
        );
    }

    // 2. Dynamic mouth & angry brows from SlimeModel emotions
    for (sm, mut tf) in &mut mouths {
        if let Some(actor) = sim.model.actors.get(&sm.actor_id) {
            if let ActorKind::Slime(ref s) = actor.kind {
                let hunger = s.hunger.curr_val;
                let fear = s.fear.curr_val;
                let agi = s.agitation.curr_val;
                if fear > 0.45 {
                    // Wide surprised 'O'
                    tf.scale = Vec3::new(1.0, 1.35, 0.5);
                } else if hunger > 0.65 {
                    // Hungry open mouth
                    tf.scale = Vec3::new(1.45, 1.15, 0.5);
                } else if agi > 0.55 || s.is_feral || actor.ident == IdentifiableId::TarrSlime {
                    // Snarl
                    tf.scale = Vec3::new(1.6, 0.45, 0.45);
                } else {
                    // Happy smile
                    tf.scale = Vec3::new(1.25, 0.55, 0.4);
                }
            }
        }
    }

    for (sb, mut vis) in &mut brows {
        let mut angry = false;
        if let Some(actor) = sim.model.actors.get(&sb.actor_id) {
            if actor.ident == IdentifiableId::TarrSlime {
                angry = true;
            } else if let ActorKind::Slime(ref s) = actor.kind {
                angry = s.is_feral || s.agitation.curr_val > 0.55;
            }
        }
        let target_vis = if angry {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *vis != target_vis {
            *vis = target_vis;
        }
    }

    // 3. Pulsing Rad / Tarr auras
    for (aura, mut tf) in &mut auras {
        let speed = if aura.is_tarr { 6.0 } else { 3.2 };
        let pulse = 1.0 + 0.08 * (t * speed + aura.actor_id as f32).sin();
        tf.scale = Vec3::splat(pulse);
    }

    // 4. Gordo breathing animation
    for (g, mut tf) in &mut gordos {
        let pulse = (t * 2.1).sin();
        let y_scale = 2.15 + 0.06 * pulse;
        let xz_scale = 2.15 - 0.03 * pulse;
        tf.scale = Vec3::new(xz_scale, y_scale, xz_scale);
        tf.translation = g.base_pos + Vec3::Y * (0.04 * pulse);
    }

    // 5. Rotating floating SlimeKey preview above Gordo
    for mut tf in &mut keys {
        tf.rotation = Quat::from_rotation_y(t * 1.8);
        tf.translation.y = 1.45 + 0.08 * (t * 2.5).sin();
    }
}
