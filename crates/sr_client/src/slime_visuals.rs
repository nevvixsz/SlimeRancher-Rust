//! Original-style Slime Rancher visuals:
//! - Original Unity `slime_default.obj` / `slime_puddle_LOD0.obj` / `slime_gordo.obj` / `slime_tarr.obj` /
//!   `slime_saber.obj` / `slime_gold.obj` / `slime_quicksilver.obj` meshes when present
//! - 3-color vertical jelly palette gradients (`Top` / `Middle` / `Bottom`)
//! - Surface-conforming face decals (`face_default_*.png` or crisp anti-aliased `eyes` + `mouth`)
//!   driven by `SlimeFaceAnimator` states (`Happy`, `Blink`, `Elated`, `Hungry`, `Starving`, `Angry`, `Feral`, `Scared`)
//! - Original aligned 3D species attachments (`ears_n_tail_LOD0`, `hunter_ears_n_tail_LOD0`, `rocks_spine_LOD0`,
//!   `crystals_spine_LOD0`, `wing_l`/`wing_r`, `dervishRing_LOD0`, `tangleFlower_LOD0`, `luckycat_coin_LOD0`,
//!   `quicksilvercrest`, `mosaic_LOD0`)
//! - Food map+mask+ramp texture composition.

use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use sr_ids::IdentifiableId;
use sr_model::ActorKind;

use crate::assets::{build_food_composite_image, ClientAssets, SlimeExpr};
use crate::obj_loader::{build_surface_patch_from_mesh, load_obj_aligned_from_disk};
use crate::world_sync::{SimResource, VisualRegistry};

#[derive(Component)]
pub struct SlimeBodyVisual {
    pub actor_id: i64,
    pub base_scale: f32,
    pub is_puddle: bool,
}

#[derive(Component)]
pub struct SlimeFullFaceVisual {
    pub actor_id: i64,
    pub current: SlimeExpr,
    pub is_tarr: bool,
}

#[derive(Component)]
pub struct SlimeEyesVisual {
    pub actor_id: i64,
    pub current: SlimeExpr,
    pub is_tarr: bool,
}

#[derive(Component)]
pub struct SlimeMouthVisual {
    pub actor_id: i64,
    pub current: SlimeExpr,
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
    pub slime_default_mesh: Handle<Mesh>,
    pub slime_puddle_mesh: Handle<Mesh>,
    pub slime_gordo_mesh: Handle<Mesh>,
    pub slime_tarr_mesh: Handle<Mesh>,
    pub slime_saber_mesh: Handle<Mesh>,
    pub slime_gold_mesh: Handle<Mesh>,
    pub slime_quicksilver_mesh: Handle<Mesh>,

    pub use_full_face_decal: bool,
    pub full_face_patch_mesh: Handle<Mesh>,
    pub eyes_patch_mesh: Handle<Mesh>,
    pub mouth_patch_mesh: Handle<Mesh>,

    // Original aligned 3D species accessories
    pub tabby_ears_mesh: Option<Handle<Mesh>>,
    pub hunter_ears_mesh: Option<Handle<Mesh>>,
    pub rock_spine_mesh: Option<Handle<Mesh>>,
    pub crystal_spine_mesh: Option<Handle<Mesh>>,
    pub phosphor_wing_l_mesh: Option<Handle<Mesh>>,
    pub phosphor_wing_r_mesh: Option<Handle<Mesh>>,
    pub dervish_ring_mesh: Option<Handle<Mesh>>,
    pub tangle_flower_mesh: Option<Handle<Mesh>>,
    pub lucky_coin_mesh: Option<Handle<Mesh>>,
    pub quicksilver_crest_mesh: Option<Handle<Mesh>>,
    pub mosaic_shell_mesh: Option<Handle<Mesh>>,

    // Fallback primitive meshes for items/extras without .obj
    pub carrot_mesh: Handle<Mesh>,
    pub leaf_mesh: Handle<Mesh>,
    pub ear_mesh: Handle<Mesh>,
    pub spike_mesh: Handle<Mesh>,
    pub wing_mesh: Handle<Mesh>,
    pub halo_mesh: Handle<Mesh>,
    pub aura_mesh: Handle<Mesh>,
    pub beak_mesh: Handle<Mesh>,
    pub key_shaft_mesh: Handle<Mesh>,

    pub slime_body_materials: HashMap<IdentifiableId, Handle<StandardMaterial>>,
    pub full_face_materials: HashMap<SlimeExpr, Handle<StandardMaterial>>,
    pub eyes_materials: HashMap<SlimeExpr, Handle<StandardMaterial>>,
    pub mouth_materials: HashMap<SlimeExpr, Handle<StandardMaterial>>,
    pub tarr_eyes_material: Handle<StandardMaterial>,
    pub tarr_mouth_material: Handle<StandardMaterial>,
    pub composed_food_textures: HashMap<IdentifiableId, Handle<Image>>,

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
    pub dervish_ring_material: Handle<StandardMaterial>,
    pub tangle_flower_material: Handle<StandardMaterial>,
    pub mosaic_shell_material: Handle<StandardMaterial>,
    pub gold_key_material: Handle<StandardMaterial>,
}

impl SlimeSharedAssets {
    pub fn body_material_for(&self, ident: IdentifiableId) -> Option<Handle<StandardMaterial>> {
        self.slime_body_materials.get(&ident).cloned()
    }

    pub fn body_mesh_for(&self, ident: IdentifiableId) -> Handle<Mesh> {
        match ident {
            IdentifiableId::PuddleSlime => self.slime_puddle_mesh.clone(),
            IdentifiableId::TarrSlime => self.slime_tarr_mesh.clone(),
            IdentifiableId::SaberSlime => self.slime_saber_mesh.clone(),
            IdentifiableId::GoldSlime => self.slime_gold_mesh.clone(),
            IdentifiableId::QuicksilverSlime => self.slime_quicksilver_mesh.clone(),
            _ => self.slime_default_mesh.clone(),
        }
    }
}

/// Top, Middle, Bottom colors matching Slime Rancher `SlimeAppearance.Palette`.
pub fn slime_palette(ident: IdentifiableId) -> ([u8; 3], [u8; 3], [u8; 3], Option<[f32; 3]>) {
    match ident {
        IdentifiableId::PinkSlime => ([255, 118, 178], [244, 62, 142], [206, 28, 102], None),
        IdentifiableId::TabbySlime => ([188, 194, 206], [142, 148, 162], [96, 102, 116], None),
        IdentifiableId::RockSlime => ([92, 152, 236], [56, 106, 198], [34, 68, 142], None),
        IdentifiableId::PhosphorSlime => (
            [170, 208, 255],
            [122, 146, 245],
            [86, 98, 212],
            Some([0.16, 0.22, 0.45]),
        ),
        IdentifiableId::HoneySlime => ([255, 208, 56], [244, 156, 22], [198, 104, 10], None),
        IdentifiableId::BoomSlime => (
            [252, 98, 52],
            [212, 46, 24],
            [132, 22, 16],
            Some([0.22, 0.04, 0.01]),
        ),
        IdentifiableId::RadSlime => (
            [152, 255, 92],
            [68, 224, 62],
            [26, 158, 42],
            Some([0.08, 0.34, 0.08]),
        ),
        IdentifiableId::CrystalSlime => (
            [252, 96, 196],
            [210, 46, 152],
            [144, 22, 108],
            Some([0.18, 0.04, 0.14]),
        ),
        IdentifiableId::HunterSlime => ([184, 122, 86], [134, 78, 52], [86, 46, 30], None),
        IdentifiableId::QuantumSlime => (
            [255, 232, 116],
            [236, 192, 48],
            [184, 134, 22],
            Some([0.20, 0.16, 0.02]),
        ),
        IdentifiableId::DervishSlime => ([198, 152, 246], [150, 96, 214], [104, 56, 168], None),
        IdentifiableId::TangleSlime => ([158, 226, 104], [108, 184, 68], [68, 132, 42], None),
        IdentifiableId::MosaicSlime => (
            [140, 224, 248],
            [184, 162, 238],
            [128, 198, 196],
            Some([0.10, 0.16, 0.22]),
        ),
        IdentifiableId::SaberSlime => ([224, 192, 128], [186, 146, 86], [138, 102, 56], None),
        IdentifiableId::PuddleSlime => ([122, 224, 255], [76, 188, 242], [44, 142, 208], None),
        IdentifiableId::FireSlime => (
            [255, 196, 56],
            [248, 112, 22],
            [196, 42, 10],
            Some([0.38, 0.12, 0.02]),
        ),
        IdentifiableId::GoldSlime => (
            [255, 236, 98],
            [244, 192, 28],
            [194, 138, 12],
            Some([0.24, 0.18, 0.02]),
        ),
        IdentifiableId::LuckySlime => ([238, 234, 216], [204, 198, 178], [164, 158, 140], None),
        IdentifiableId::QuicksilverSlime => ([218, 228, 244], [168, 182, 206], [116, 132, 158], None),
        IdentifiableId::GlitchSlime => (
            [248, 86, 212],
            [116, 216, 248],
            [168, 62, 224],
            Some([0.22, 0.08, 0.24]),
        ),
        IdentifiableId::TarrSlime => (
            [56, 26, 74],
            [26, 28, 42],
            [12, 12, 18],
            Some([0.12, 0.04, 0.16]),
        ),
        _ => ([228, 150, 212], [200, 118, 184], [156, 84, 142], None),
    }
}

fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t).round().clamp(0.0, 255.0) as u8
}

fn create_gradient_texture(
    images: &mut Assets<Image>,
    top: [u8; 3],
    mid: [u8; 3],
    bot: [u8; 3],
) -> Handle<Image> {
    let h = 64_usize;
    let mut data = Vec::with_capacity(h * 4);
    for y in 0..h {
        let v = y as f32 / (h - 1) as f32;
        let (r, g, b) = if v < 0.5 {
            let t = v * 2.0;
            (lerp_u8(top[0], mid[0], t), lerp_u8(top[1], mid[1], t), lerp_u8(top[2], mid[2], t))
        } else {
            let t = (v - 0.5) * 2.0;
            (lerp_u8(mid[0], bot[0], t), lerp_u8(mid[1], bot[1], t), lerp_u8(mid[2], bot[2], t))
        };
        data.extend_from_slice(&[r, g, b, 255]);
    }
    images.add(Image::new(
        Extent3d {
            width: 1,
            height: h as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    ))
}

fn blend_px(buf: &mut [u8], w: usize, x: i32, y: i32, rgba: [u8; 4]) {
    if x < 0 || y < 0 || x >= w as i32 {
        return;
    }
    let idx = (y as usize * w + x as usize) * 4;
    if idx + 3 >= buf.len() {
        return;
    }
    let a = rgba[3] as f32 / 255.0;
    let inv = 1.0 - a;
    buf[idx] = (buf[idx] as f32 * inv + rgba[0] as f32 * a).round() as u8;
    buf[idx + 1] = (buf[idx + 1] as f32 * inv + rgba[1] as f32 * a).round() as u8;
    buf[idx + 2] = (buf[idx + 2] as f32 * inv + rgba[2] as f32 * a).round() as u8;
    buf[idx + 3] = (buf[idx + 3].max(rgba[3]))
        .max(((buf[idx + 3] as f32) + (255.0 - buf[idx + 3] as f32) * a).round() as u8);
}

fn draw_filled_ellipse(
    buf: &mut [u8],
    w: usize,
    h: usize,
    cx: f32,
    cy: f32,
    rx: f32,
    ry: f32,
    rgb: [u8; 3],
) {
    for y in 0..h {
        for x in 0..w {
            let dx = (x as f32 + 0.5 - cx) / rx;
            let dy = (y as f32 + 0.5 - cy) / ry;
            let d = (dx * dx + dy * dy).sqrt();
            if d < 1.08 {
                let alpha = ((1.08 - d) / 0.14).clamp(0.0, 1.0);
                blend_px(buf, w, x as i32, y as i32, [rgb[0], rgb[1], rgb[2], (alpha * 255.0) as u8]);
            }
        }
    }
}

fn draw_arc_stroke(
    buf: &mut [u8],
    w: usize,
    h: usize,
    cx: f32,
    cy: f32,
    rx: f32,
    ry: f32,
    upward_curve: bool,
    thickness: f32,
    rgb: [u8; 3],
) {
    for y in 0..h {
        for x in 0..w {
            let nx = (x as f32 + 0.5 - cx) / rx;
            if nx.abs() > 1.05 {
                continue;
            }
            let curve = (1.0 - nx.clamp(-1.0, 1.0).powi(2)).max(0.0).sqrt();
            let target_y = if upward_curve {
                cy - curve * ry
            } else {
                cy + curve * ry
            };
            let dist = ((y as f32 + 0.5) - target_y).abs();
            if dist < thickness + 1.2 {
                let alpha = ((thickness + 1.2 - dist) / 1.4).clamp(0.0, 1.0);
                blend_px(buf, w, x as i32, y as i32, [rgb[0], rgb[1], rgb[2], (alpha * 255.0) as u8]);
            }
        }
    }
}

fn create_procedural_eyes_image(images: &mut Assets<Image>, expr: SlimeExpr, is_tarr: bool) -> Handle<Image> {
    let (w, h) = (128_usize, 64_usize);
    let mut buf = vec![0_u8; w * h * 4];
    let eye_dark = if is_tarr { [255, 255, 255] } else { [34, 14, 24] };
    let left_cx = 34.0_f32;
    let right_cx = 94.0_f32;
    let cy = 34.0_f32;

    match expr {
        SlimeExpr::Blink => {
            for cx in [left_cx, right_cx] {
                draw_arc_stroke(&mut buf, w, h, cx, cy + 4.0, 13.0, 6.0, false, 2.8, eye_dark);
            }
        }
        SlimeExpr::Elated => {
            for cx in [left_cx, right_cx] {
                draw_arc_stroke(&mut buf, w, h, cx, cy + 5.0, 13.0, 10.0, true, 3.0, eye_dark);
            }
        }
        SlimeExpr::Angry | SlimeExpr::Feral => {
            for (idx, cx) in [left_cx, right_cx].iter().enumerate() {
                draw_filled_ellipse(&mut buf, w, h, *cx, cy + 3.0, 10.0, 11.5, eye_dark);
                if !is_tarr {
                    draw_filled_ellipse(&mut buf, w, h, *cx - 2.8, cy - 1.5, 3.6, 4.0, [255, 255, 255]);
                }
                let dir = if idx == 0 { 1.0 } else { -1.0 };
                for step in -12..=12 {
                    let px = *cx + step as f32;
                    let py = cy - 9.0 + dir * (step as f32 * 0.42);
                    draw_filled_ellipse(&mut buf, w, h, px, py, 2.5, 2.5, eye_dark);
                }
            }
        }
        SlimeExpr::Scared => {
            for cx in [left_cx, right_cx] {
                draw_filled_ellipse(&mut buf, w, h, cx, cy, 13.0, 14.5, [255, 255, 255]);
                draw_filled_ellipse(&mut buf, w, h, cx, cy + 1.0, 5.2, 5.8, eye_dark);
            }
        }
        _ => {
            for cx in [left_cx, right_cx] {
                draw_filled_ellipse(&mut buf, w, h, cx, cy + 2.0, 10.5, 13.0, eye_dark);
                if !is_tarr {
                    draw_filled_ellipse(&mut buf, w, h, cx - 3.2, cy - 3.0, 4.0, 4.6, [255, 255, 255]);
                    draw_filled_ellipse(&mut buf, w, h, cx + 3.4, cy + 4.5, 1.8, 2.0, [255, 255, 255]);
                }
            }
        }
    }

    images.add(Image::new(
        Extent3d {
            width: w as u32,
            height: h as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        buf,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    ))
}

fn create_procedural_mouth_image(images: &mut Assets<Image>, expr: SlimeExpr, is_tarr: bool) -> Handle<Image> {
    let (w, h) = (128_usize, 64_usize);
    let mut buf = vec![0_u8; w * h * 4];
    let mouth_dark = if is_tarr { [255, 255, 255] } else { [92, 18, 42] };
    let tongue = [232, 92, 128];
    let cx = 64.0_f32;
    let cy = 28.0_f32;

    match expr {
        SlimeExpr::Happy | SlimeExpr::Blink => {
            draw_arc_stroke(&mut buf, w, h, cx, cy - 4.0, 19.0, 11.0, false, 3.0, mouth_dark);
        }
        SlimeExpr::Elated | SlimeExpr::Hungry | SlimeExpr::Starving | SlimeExpr::ChompOpen => {
            for y in 0..h {
                for x in 0..w {
                    let dx = (x as f32 + 0.5 - cx) / 22.0;
                    let dy = (y as f32 + 0.5 - (cy - 4.0)) / 20.0;
                    if dy >= -0.08 && (dx * dx + dy * dy) <= 1.0 {
                        let is_tongue = dy > 0.42 && dx.abs() < 0.68;
                        let c = if is_tongue && !is_tarr { tongue } else { mouth_dark };
                        blend_px(&mut buf, w, x as i32, y as i32, [c[0], c[1], c[2], 255]);
                    }
                }
            }
        }
        SlimeExpr::Angry | SlimeExpr::Feral => {
            draw_arc_stroke(&mut buf, w, h, cx, cy + 10.0, 18.0, 10.0, true, 3.0, mouth_dark);
        }
        SlimeExpr::Scared => {
            draw_filled_ellipse(&mut buf, w, h, cx, cy + 4.0, 9.0, 11.0, mouth_dark);
        }
    }

    images.add(Image::new(
        Extent3d {
            width: w as u32,
            height: h as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        buf,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    ))
}

pub fn init_slime_shared_assets(
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    assets: &ClientAssets,
) -> SlimeSharedAssets {
    // 1. Load original slime_default.obj (and detect whether AssetStudio exported it facing -Z)
    let fallback_sphere = Sphere::new(1.0).mesh().uv(32, 20);
    let mut flip_y_180 = false;

    if let (Some(body_p), Some(face_p)) = (
        assets.slime_obj_paths.get("slime_default"),
        assets.slime_obj_paths.get("slime_face_lod0"),
    ) {
        if let Some((_, body_b)) = load_obj_aligned_from_disk(body_p, None, false, false) {
            if let Some((_, face_b)) = load_obj_aligned_from_disk(face_p, None, false, false) {
                if face_b.center.z < body_b.center.z {
                    flip_y_180 = true;
                }
            }
        }
    }

    let (base_body_mesh, base_align) = assets
        .slime_obj_paths
        .get("slime_default")
        .and_then(|p| load_obj_aligned_from_disk(p, None, true, flip_y_180))
        .map(|(m, b)| (m, Some((b.center, b.scale))))
        .unwrap_or_else(|| (fallback_sphere.clone(), None));

    let load_aligned_body = |key: &str| -> Mesh {
        assets
            .slime_obj_paths
            .get(key)
            .and_then(|p| load_obj_aligned_from_disk(p, None, true, flip_y_180))
            .map(|(m, _)| m)
            .unwrap_or_else(|| base_body_mesh.clone())
    };
    let puddle_raw = load_aligned_body("slime_puddle_lod0");
    let gordo_raw = load_aligned_body("slime_gordo");
    let tarr_raw = load_aligned_body("slime_tarr");
    let saber_raw = load_aligned_body("slime_saber");
    let gold_raw = load_aligned_body("slime_gold");
    let quicksilver_raw = load_aligned_body("slime_quicksilver");

    // Conform face patches closely to the real slime_default surface!
    let full_face_patch_mesh = meshes.add(build_surface_patch_from_mesh(
        &base_body_mesh,
        0.020,
        -0.55,
        0.55,
        -0.36,
        0.44,
        12,
        10,
    ));
    let eyes_patch_mesh = meshes.add(build_surface_patch_from_mesh(
        &base_body_mesh,
        0.018,
        -0.52,
        0.52,
        -0.02,
        0.44,
        10,
        6,
    ));
    let mouth_patch_mesh = meshes.add(build_surface_patch_from_mesh(
        &base_body_mesh,
        0.020,
        -0.36,
        0.36,
        -0.38,
        0.04,
        8,
        5,
    ));

    let slime_default_mesh = meshes.add(base_body_mesh);
    let slime_puddle_mesh = meshes.add(puddle_raw);
    let slime_gordo_mesh = meshes.add(gordo_raw);
    let slime_tarr_mesh = meshes.add(tarr_raw);
    let slime_saber_mesh = meshes.add(saber_raw);
    let slime_gold_mesh = meshes.add(gold_raw);
    let slime_quicksilver_mesh = meshes.add(quicksilver_raw);

    let load_part_raw = |key: &str| -> Option<Mesh> {
        let path = assets.slime_obj_paths.get(key)?;
        let (mesh, _) = load_obj_aligned_from_disk(path, base_align, false, flip_y_180)?;
        Some(mesh)
    };
    let tabby_ears_mesh = load_part_raw("ears_n_tail_lod0").map(|m| meshes.add(m));
    let hunter_ears_mesh = load_part_raw("hunter_ears_n_tail_lod0").map(|m| meshes.add(m));
    let rock_spine_mesh = load_part_raw("rocks_spine_lod0").map(|m| meshes.add(m));
    let crystal_spine_mesh = load_part_raw("crystals_spine_lod0").map(|m| meshes.add(m));
    let phosphor_wing_l_mesh = load_part_raw("wing_l").map(|m| meshes.add(m));
    let phosphor_wing_r_mesh = load_part_raw("wing_r").map(|m| meshes.add(m));
    let dervish_ring_mesh = load_part_raw("dervishring_lod0").map(|m| meshes.add(m));
    let tangle_flower_mesh = load_part_raw("tangleflower_lod0").map(|m| meshes.add(m));
    let lucky_coin_mesh = load_part_raw("luckycat_coin_lod0").map(|m| meshes.add(m));
    let quicksilver_crest_mesh = load_part_raw("quicksilvercrest").map(|m| meshes.add(m));
    let mosaic_shell_mesh = load_part_raw("mosaic_lod0").map(|m| meshes.add(m));

    // 2. 3-color vertical jelly materials for every slime species
    let mut slime_body_materials = HashMap::new();
    for &id in IdentifiableId::ALL {
        if id.is_slime() {
            let (top, mid, bot, emissive) = slime_palette(id);
            let grad = create_gradient_texture(images, top, mid, bot);
            let is_puddle = id == IdentifiableId::PuddleSlime;
            let mat = materials.add(StandardMaterial {
                base_color: if is_puddle {
                    Color::srgba(1.0, 1.0, 1.0, 0.85)
                } else {
                    Color::WHITE
                },
                base_color_texture: Some(grad),
                emissive: emissive
                    .map(|e| LinearRgba::rgb(e[0], e[1], e[2]))
                    .unwrap_or(LinearRgba::BLACK),
                alpha_mode: if is_puddle { AlphaMode::Blend } else { AlphaMode::Opaque },
                perceptual_roughness: 0.14,
                metallic: 0.04,
                ..default()
            });
            slime_body_materials.insert(id, mat);
        }
    }

    // 3. Original combined face textures (`face_default_*.png`) if validated transparent, plus fallback split decals
    let use_full_face_decal = !assets.face_full_textures.is_empty();
    let mut full_face_materials = HashMap::new();
    if use_full_face_decal {
        let happy_fb = assets
            .face_full_textures
            .get(&SlimeExpr::Happy)
            .cloned()
            .unwrap_or_else(|| assets.face_full_textures.values().next().unwrap().clone());
        for &expr in SlimeExpr::ALL {
            let tex = assets
                .face_full_textures
                .get(&expr)
                .cloned()
                .unwrap_or_else(|| happy_fb.clone());
            full_face_materials.insert(
                expr,
                materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    base_color_texture: Some(tex),
                    alpha_mode: AlphaMode::Blend,
                    unlit: true,
                    ..default()
                }),
            );
        }
    }

    let mut eyes_materials = HashMap::new();
    let mut mouth_materials = HashMap::new();
    for &expr in SlimeExpr::ALL {
        let eyes_tex = assets
            .face_eyes_textures
            .get(&expr)
            .cloned()
            .unwrap_or_else(|| create_procedural_eyes_image(images, expr, false));
        let mouth_tex = assets
            .face_mouth_textures
            .get(&expr)
            .cloned()
            .unwrap_or_else(|| create_procedural_mouth_image(images, expr, false));
        eyes_materials.insert(
            expr,
            materials.add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(eyes_tex),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            }),
        );
        mouth_materials.insert(
            expr,
            materials.add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(mouth_tex),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            }),
        );
    }

    let tarr_eyes_tex = create_procedural_eyes_image(images, SlimeExpr::Angry, true);
    let tarr_mouth_tex = create_procedural_mouth_image(images, SlimeExpr::Feral, true);
    let tarr_eyes_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.90, 1.0),
        base_color_texture: Some(tarr_eyes_tex),
        emissive: LinearRgba::rgb(0.65, 0.25, 0.85),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });
    let tarr_mouth_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.90, 1.0),
        base_color_texture: Some(tarr_mouth_tex),
        emissive: LinearRgba::rgb(0.65, 0.25, 0.85),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });

    // 4. Compose vivid food textures from `_map` + `_mask` + `_ramp`
    let mut composed_food_textures = HashMap::new();
    for spec in &assets.food_composites {
        if let Some(img) = build_food_composite_image(spec) {
            composed_food_textures.insert(spec.id, images.add(img));
        }
    }

    SlimeSharedAssets {
        slime_default_mesh,
        slime_puddle_mesh,
        slime_gordo_mesh,
        slime_tarr_mesh,
        slime_saber_mesh,
        slime_gold_mesh,
        slime_quicksilver_mesh,

        use_full_face_decal,
        full_face_patch_mesh,
        eyes_patch_mesh,
        mouth_patch_mesh,

        tabby_ears_mesh,
        hunter_ears_mesh,
        rock_spine_mesh,
        crystal_spine_mesh,
        phosphor_wing_l_mesh,
        phosphor_wing_r_mesh,
        dervish_ring_mesh,
        tangle_flower_mesh,
        lucky_coin_mesh,
        quicksilver_crest_mesh,
        mosaic_shell_mesh,

        carrot_mesh: meshes.add(Cone::new(0.32, 1.15)),
        leaf_mesh: meshes.add(Cone::new(0.20, 0.45)),
        ear_mesh: meshes.add(Cone::new(0.24, 0.55)),
        spike_mesh: meshes.add(Cone::new(0.22, 0.62)),
        wing_mesh: meshes.add(Cuboid::new(0.65, 0.08, 0.34)),
        halo_mesh: meshes.add(Torus::new(0.26, 0.36)),
        aura_mesh: meshes.add(Sphere::new(1.55).mesh().ico(2).unwrap()),
        beak_mesh: meshes.add(Cone::new(0.16, 0.36)),
        key_shaft_mesh: meshes.add(Cylinder::new(0.08, 0.65)),

        slime_body_materials,
        full_face_materials,
        eyes_materials,
        mouth_materials,
        tarr_eyes_material,
        tarr_mouth_material,
        composed_food_textures,

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
        dervish_ring_material: materials.add(StandardMaterial {
            base_color: Color::srgba(0.84, 0.62, 1.0, 0.85),
            emissive: LinearRgba::rgb(0.22, 0.10, 0.38),
            alpha_mode: AlphaMode::Blend,
            perceptual_roughness: 0.2,
            ..default()
        }),
        tangle_flower_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.98, 0.48, 0.55),
            perceptual_roughness: 0.45,
            ..default()
        }),
        mosaic_shell_material: materials.add(StandardMaterial {
            base_color: Color::srgba(0.75, 0.94, 1.0, 0.45),
            emissive: LinearRgba::rgb(0.14, 0.20, 0.28),
            alpha_mode: AlphaMode::Blend,
            perceptual_roughness: 0.08,
            metallic: 0.35,
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
        IdentifiableId::TabbySlime => {
            if let Some(ref m) = shared.tabby_ears_mesh {
                parent.spawn((Mesh3d(m.clone()), MeshMaterial3d(body_mat), Transform::IDENTITY));
            } else {
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
        }
        IdentifiableId::HunterSlime => {
            if let Some(ref m) = shared.hunter_ears_mesh.as_ref().or(shared.tabby_ears_mesh.as_ref()) {
                parent.spawn((Mesh3d((*m).clone()), MeshMaterial3d(body_mat), Transform::IDENTITY));
            } else {
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
        }
        IdentifiableId::LuckySlime => {
            if let Some(ref m) = shared.tabby_ears_mesh {
                parent.spawn((Mesh3d(m.clone()), MeshMaterial3d(body_mat), Transform::IDENTITY));
            }
            if let Some(ref coin) = shared.lucky_coin_mesh {
                parent.spawn((
                    Mesh3d(coin.clone()),
                    MeshMaterial3d(shared.gold_key_material.clone()),
                    Transform::IDENTITY,
                ));
            }
        }
        IdentifiableId::RockSlime => {
            if let Some(ref m) = shared.rock_spine_mesh {
                parent.spawn((
                    Mesh3d(m.clone()),
                    MeshMaterial3d(shared.rock_spike_material.clone()),
                    Transform::IDENTITY,
                ));
            } else {
                for (x, y, z, rx, rz) in [
                    (0.0_f32, 1.02_f32, 0.0_f32, 0.0_f32, 0.0_f32),
                    (-0.48, 0.82, -0.15, -0.25, 0.45),
                    (0.48, 0.82, -0.15, -0.25, -0.45),
                ] {
                    parent.spawn((
                        Mesh3d(shared.spike_mesh.clone()),
                        MeshMaterial3d(shared.rock_spike_material.clone()),
                        Transform::from_xyz(x, y, z)
                            .with_rotation(Quat::from_euler(EulerRot::XYZ, rx, 0.0, rz)),
                    ));
                }
            }
        }
        IdentifiableId::CrystalSlime => {
            if let Some(ref m) = shared.crystal_spine_mesh {
                parent.spawn((
                    Mesh3d(m.clone()),
                    MeshMaterial3d(shared.crystal_spike_material.clone()),
                    Transform::IDENTITY,
                ));
            } else {
                for (x, y, z, rx, rz) in [
                    (0.0_f32, 1.05_f32, 0.0_f32, 0.0_f32, 0.0_f32),
                    (-0.52, 0.85, 0.0, 0.0, 0.55),
                    (0.52, 0.85, 0.0, 0.0, -0.55),
                ] {
                    parent.spawn((
                        Mesh3d(shared.spike_mesh.clone()),
                        MeshMaterial3d(shared.crystal_spike_material.clone()),
                        Transform::from_xyz(x, y, z)
                            .with_rotation(Quat::from_euler(EulerRot::XYZ, rx, 0.0, rz)),
                    ));
                }
            }
        }
        IdentifiableId::PhosphorSlime => {
            if let (Some(ref wl), Some(ref wr)) =
                (&shared.phosphor_wing_l_mesh, &shared.phosphor_wing_r_mesh)
            {
                parent.spawn((
                    Mesh3d(wl.clone()),
                    MeshMaterial3d(shared.phosphor_wing_material.clone()),
                    Transform::IDENTITY,
                ));
                parent.spawn((
                    Mesh3d(wr.clone()),
                    MeshMaterial3d(shared.phosphor_wing_material.clone()),
                    Transform::IDENTITY,
                ));
            } else {
                for side in [-1.0_f32, 1.0_f32] {
                    parent.spawn((
                        Mesh3d(shared.wing_mesh.clone()),
                        MeshMaterial3d(shared.phosphor_wing_material.clone()),
                        Transform::from_xyz(side * 0.92, 0.35, -0.20)
                            .with_rotation(Quat::from_rotation_z(side * 0.35)),
                    ));
                }
            }
            parent.spawn((
                Mesh3d(shared.halo_mesh.clone()),
                MeshMaterial3d(shared.phosphor_wing_material.clone()),
                Transform::from_xyz(0.0, 1.25, 0.0),
            ));
        }
        IdentifiableId::DervishSlime => {
            if let Some(ref m) = shared.dervish_ring_mesh {
                parent.spawn((
                    Mesh3d(m.clone()),
                    MeshMaterial3d(shared.dervish_ring_material.clone()),
                    Transform::IDENTITY,
                ));
            }
        }
        IdentifiableId::TangleSlime => {
            if let Some(ref m) = shared.tangle_flower_mesh {
                parent.spawn((
                    Mesh3d(m.clone()),
                    MeshMaterial3d(shared.tangle_flower_material.clone()),
                    Transform::IDENTITY,
                ));
            }
        }
        IdentifiableId::MosaicSlime => {
            if let Some(ref m) = shared.mosaic_shell_mesh {
                parent.spawn((
                    Mesh3d(m.clone()),
                    MeshMaterial3d(shared.mosaic_shell_material.clone()),
                    Transform::IDENTITY,
                ));
            }
        }
        IdentifiableId::QuicksilverSlime => {
            if let Some(ref m) = shared.quicksilver_crest_mesh {
                parent.spawn((
                    Mesh3d(m.clone()),
                    MeshMaterial3d(body_mat),
                    Transform::IDENTITY,
                ));
            }
        }
        IdentifiableId::HoneySlime => {
            parent.spawn((
                Mesh3d(shared.leaf_mesh.clone()),
                MeshMaterial3d(shared.honey_cap_material.clone()),
                Transform::from_xyz(0.0, 0.95, -0.05).with_scale(Vec3::new(1.8, 0.65, 1.8)),
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

fn spawn_face_decals(parent: &mut ChildBuilder, shared: &SlimeSharedAssets, is_tarr: bool) {
    if is_tarr || !shared.use_full_face_decal {
        let eyes_mat = if is_tarr {
            shared.tarr_eyes_material.clone()
        } else {
            shared.eyes_materials[&SlimeExpr::Happy].clone()
        };
        let mouth_mat = if is_tarr {
            shared.tarr_mouth_material.clone()
        } else {
            shared.mouth_materials[&SlimeExpr::Happy].clone()
        };
        parent.spawn((
            Mesh3d(shared.eyes_patch_mesh.clone()),
            MeshMaterial3d(eyes_mat),
            Transform::IDENTITY,
            SlimeEyesVisual {
                actor_id: 0,
                current: SlimeExpr::Happy,
                is_tarr,
            },
        ));
        parent.spawn((
            Mesh3d(shared.mouth_patch_mesh.clone()),
            MeshMaterial3d(mouth_mat),
            Transform::IDENTITY,
            SlimeMouthVisual {
                actor_id: 0,
                current: SlimeExpr::Happy,
            },
        ));
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
    let is_tarr = ident == IdentifiableId::TarrSlime;

    if ident.is_slime() {
        let body_mat = shared.body_material_for(ident).unwrap_or(mat_handle);
        let body_mesh = shared.body_mesh_for(ident);
        let scale = if is_puddle {
            Vec3::new(radius * 1.15, radius * 0.72, radius * 1.15)
        } else {
            Vec3::splat(radius)
        };

        return commands
            .spawn((
                Mesh3d(body_mesh),
                MeshMaterial3d(body_mat.clone()),
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
                if !is_tarr && shared.use_full_face_decal {
                    parent.spawn((
                        Mesh3d(shared.full_face_patch_mesh.clone()),
                        MeshMaterial3d(shared.full_face_materials[&SlimeExpr::Happy].clone()),
                        Transform::IDENTITY,
                        SlimeFullFaceVisual {
                            actor_id,
                            current: SlimeExpr::Happy,
                            is_tarr,
                        },
                    ));
                } else {
                    let eyes_mat = if is_tarr {
                        shared.tarr_eyes_material.clone()
                    } else {
                        shared.eyes_materials[&SlimeExpr::Happy].clone()
                    };
                    let mouth_mat = if is_tarr {
                        shared.tarr_mouth_material.clone()
                    } else {
                        shared.mouth_materials[&SlimeExpr::Happy].clone()
                    };
                    parent.spawn((
                        Mesh3d(shared.eyes_patch_mesh.clone()),
                        MeshMaterial3d(eyes_mat),
                        Transform::IDENTITY,
                        SlimeEyesVisual {
                            actor_id,
                            current: SlimeExpr::Happy,
                            is_tarr,
                        },
                    ));
                    parent.spawn((
                        Mesh3d(shared.mouth_patch_mesh.clone()),
                        MeshMaterial3d(mouth_mat),
                        Transform::IDENTITY,
                        SlimeMouthVisual {
                            actor_id,
                            current: SlimeExpr::Happy,
                        },
                    ));
                }

                spawn_slime_species_traits(parent, ident, shared, body_mat);

                if ident == IdentifiableId::RadSlime || is_tarr {
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

    let species_mesh = assets.item_meshes.get(&ident).cloned();

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
                    scale: Vec3::splat(radius * 1.05),
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
            shared.slime_default_mesh.clone()
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
                Mesh3d(shared.slime_default_mesh.clone()),
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
                Mesh3d(shared.slime_default_mesh.clone()),
                MeshMaterial3d(mat_handle),
                Transform {
                    translation: pos,
                    rotation: rot,
                    scale: Vec3::new(radius * 0.95, radius * 0.88, radius * 1.15),
                },
            ))
            .with_children(|parent| {
                parent.spawn((
                    Mesh3d(shared.beak_mesh.clone()),
                    MeshMaterial3d(shared.beak_material.clone()),
                    Transform::from_xyz(0.0, 0.10, 1.05)
                        .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                ));
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

    commands
        .spawn((
            Mesh3d(shared.slime_default_mesh.clone()),
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
    shared: &SlimeSharedAssets,
    gordo_id: String,
    slime_ident: IdentifiableId,
    pos: Vec3,
    fallback_mat: Handle<StandardMaterial>,
) -> Entity {
    let base_pos = pos + Vec3::Y * 2.05;
    let body_mat = shared.body_material_for(slime_ident).unwrap_or(fallback_mat);
    commands
        .spawn((
            Mesh3d(shared.slime_gordo_mesh.clone()),
            MeshMaterial3d(body_mat.clone()),
            Transform::from_translation(base_pos).with_scale(Vec3::splat(2.15)),
            GordoBodyVisual { gordo_id, base_pos },
        ))
        .with_children(|parent| {
            if shared.use_full_face_decal {
                parent.spawn((
                    Mesh3d(shared.full_face_patch_mesh.clone()),
                    MeshMaterial3d(shared.full_face_materials[&SlimeExpr::Elated].clone()),
                    Transform::IDENTITY,
                ));
            } else {
                spawn_face_decals(parent, shared, false);
            }
            spawn_slime_species_traits(parent, slime_ident, shared, body_mat);

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

fn evaluate_slime_expression(
    sim: &sr_sim::Simulation,
    actor_id: i64,
    elapsed: f32,
) -> (SlimeExpr, SlimeExpr) {
    let Some(actor) = sim.model.actors.get(&actor_id) else {
        return (SlimeExpr::Happy, SlimeExpr::Happy);
    };
    if actor.ident == IdentifiableId::TarrSlime {
        return (SlimeExpr::Angry, SlimeExpr::Feral);
    }
    let ActorKind::Slime(ref s) = actor.kind else {
        return (SlimeExpr::Happy, SlimeExpr::Happy);
    };
    let agi = s.agitation.curr_val;
    let hunger = s.hunger.curr_val;
    let fear = s.fear.curr_val;

    let base_expr = if s.is_feral {
        SlimeExpr::Feral
    } else if agi > 0.78 {
        SlimeExpr::Angry
    } else if fear > 0.55 {
        SlimeExpr::Scared
    } else if hunger > 0.85 {
        SlimeExpr::Starving
    } else if hunger > 0.60 {
        SlimeExpr::Hungry
    } else if agi < 0.15 && hunger < 0.35 && fear < 0.10 {
        SlimeExpr::Elated
    } else {
        SlimeExpr::Happy
    };

    let blink_cycle = (elapsed + (actor_id as f32 * 0.67).abs()) % 3.4;
    let eyes_expr = if matches!(base_expr, SlimeExpr::Happy | SlimeExpr::Elated | SlimeExpr::Feral)
        && blink_cycle < 0.14
    {
        SlimeExpr::Blink
    } else {
        base_expr
    };
    (eyes_expr, base_expr)
}

#[allow(clippy::too_many_arguments)]
pub fn update_slime_animations_system(
    time: Res<Time>,
    sim_res: Res<SimResource>,
    shared: Res<SlimeSharedAssets>,
    mut bodies: Query<
        (&SlimeBodyVisual, &mut Transform),
        (Without<GordoBodyVisual>, Without<SlimeAuraVisual>, Without<GordoKeyVisual>),
    >,
    mut full_faces_q: Query<
        (&mut SlimeFullFaceVisual, &mut MeshMaterial3d<StandardMaterial>),
        (Without<SlimeEyesVisual>, Without<SlimeMouthVisual>),
    >,
    mut eyes_q: Query<
        (&mut SlimeEyesVisual, &mut MeshMaterial3d<StandardMaterial>),
        (Without<SlimeMouthVisual>, Without<SlimeFullFaceVisual>),
    >,
    mut mouths_q: Query<
        (&mut SlimeMouthVisual, &mut MeshMaterial3d<StandardMaterial>),
        (Without<SlimeEyesVisual>, Without<SlimeFullFaceVisual>),
    >,
    mut auras: Query<
        (&SlimeAuraVisual, &mut Transform),
        (Without<SlimeBodyVisual>, Without<GordoBodyVisual>, Without<GordoKeyVisual>),
    >,
    mut gordos: Query<
        (&GordoBodyVisual, &mut Transform),
        (Without<SlimeBodyVisual>, Without<SlimeAuraVisual>, Without<GordoKeyVisual>),
    >,
    mut keys: Query<
        &mut Transform,
        (
            With<GordoKeyVisual>,
            Without<SlimeBodyVisual>,
            Without<SlimeAuraVisual>,
            Without<GordoBodyVisual>,
        ),
    >,
) {
    let t = time.elapsed_secs();
    let sim = &sim_res.sim;

    for (sb, mut tf) in &mut bodies {
        let vy = sim
            .physics
            .bodies
            .get(&sb.actor_id)
            .map(|b| b.vel.y)
            .unwrap_or(0.0);
        let stretch = (vy * 0.035).clamp(-0.22, 0.28);
        let idle = (t * 4.5 + sb.actor_id as f32 * 0.7).sin() * 0.035;
        let y_factor = (if sb.is_puddle { 0.72 } else { 1.0 }) + stretch + idle;
        let xz_factor = (if sb.is_puddle { 1.15 } else { 1.0 }) - (stretch + idle) * 0.45;
        tf.scale = Vec3::new(
            sb.base_scale * xz_factor.max(0.65),
            sb.base_scale * y_factor.max(0.50),
            sb.base_scale * xz_factor.max(0.65),
        );
    }

    for (mut ff, mut mat) in &mut full_faces_q {
        if ff.is_tarr {
            continue;
        }
        let (expr, _) = evaluate_slime_expression(sim, ff.actor_id, t);
        if ff.current != expr {
            ff.current = expr;
            if let Some(h) = shared.full_face_materials.get(&expr) {
                mat.0 = h.clone();
            }
        }
    }

    for (mut se, mut mat) in &mut eyes_q {
        if se.is_tarr {
            continue;
        }
        let (eyes_expr, _) = evaluate_slime_expression(sim, se.actor_id, t);
        if se.current != eyes_expr {
            se.current = eyes_expr;
            if let Some(h) = shared.eyes_materials.get(&eyes_expr) {
                mat.0 = h.clone();
            }
        }
    }

    for (mut sm, mut mat) in &mut mouths_q {
        let (_, mouth_expr) = evaluate_slime_expression(sim, sm.actor_id, t);
        if sm.current != mouth_expr {
            sm.current = mouth_expr;
            if let Some(h) = shared.mouth_materials.get(&mouth_expr) {
                mat.0 = h.clone();
            }
        }
    }

    for (aura, mut tf) in &mut auras {
        let speed = if aura.is_tarr { 6.0 } else { 3.2 };
        let pulse = 1.0 + 0.08 * (t * speed + aura.actor_id as f32).sin();
        tf.scale = Vec3::splat(pulse);
    }

    for (g, mut tf) in &mut gordos {
        let pulse = (t * 2.1).sin();
        let y_scale = 2.15 + 0.06 * pulse;
        let xz_scale = 2.15 - 0.03 * pulse;
        tf.scale = Vec3::new(xz_scale, y_scale, xz_scale);
        tf.translation = g.base_pos + Vec3::Y * (0.04 * pulse);
    }

    for mut tf in &mut keys {
        tf.rotation = Quat::from_rotation_y(t * 1.8);
        tf.translation.y = 1.45 + 0.08 * (t * 2.5).sin();
    }
}
