//! Fast asynchronous Wavefront `.obj` AssetLoader for AssetStudio-exported Unity `Mesh/*.obj` files.
//! Supports `v`, `vt`, `vn`, negative indices, quad/n-gon fan triangulation, smooth fallback normals,
//! and optional unit-radius normalization so actor meshes automatically fit their collider radius.

use std::collections::HashMap;
use std::fmt;

use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, LoadContext, RenderAssetUsages};
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ObjLoaderSettings {
    /// If true, centers the mesh around its bounding-box center and scales it so its
    /// half-extent max is 1.0 (matching unit-sphere collider scaling in `slime_visuals`).
    pub normalize_unit_radius: bool,
}

#[derive(Debug)]
pub struct ObjLoadError(pub String);

impl fmt::Display for ObjLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "OBJ load error: {}", self.0)
    }
}

impl std::error::Error for ObjLoadError {}

impl From<std::io::Error> for ObjLoadError {
    fn from(err: std::io::Error) -> Self {
        Self(err.to_string())
    }
}

impl From<std::str::Utf8Error> for ObjLoadError {
    fn from(err: std::str::Utf8Error) -> Self {
        Self(err.to_string())
    }
}

#[derive(Default)]
pub struct ObjAssetLoader;

impl AssetLoader for ObjAssetLoader {
    type Asset = Mesh;
    type Settings = ObjLoaderSettings;
    type Error = ObjLoadError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Mesh, ObjLoadError> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let text = std::str::from_utf8(&bytes)?;
        parse_obj_to_mesh(text, settings.normalize_unit_radius)
    }

    fn extensions(&self) -> &[&str] {
        &["obj"]
    }
}

pub struct ObjMeshPlugin;

impl Plugin for ObjMeshPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset_loader::<ObjAssetLoader>();
    }
}

fn resolve_obj_index(raw: &str, len: usize) -> Option<usize> {
    let idx: isize = raw.parse().ok()?;
    if idx > 0 {
        let u = (idx - 1) as usize;
        (u < len).then_some(u)
    } else if idx < 0 {
        let rev = (-idx) as usize;
        (rev <= len).then_some(len - rev)
    } else {
        None
    }
}

pub fn parse_obj_to_mesh(src: &str, normalize_unit_radius: bool) -> Result<Mesh, ObjLoadError> {
    let mut raw_pos: Vec<[f32; 3]> = Vec::with_capacity(1024);
    let mut raw_uv: Vec<[f32; 2]> = Vec::with_capacity(1024);
    let mut raw_norm: Vec<[f32; 3]> = Vec::with_capacity(1024);

    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(1024);
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(1024);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(1024);
    let mut indices: Vec<u32> = Vec::with_capacity(2048);
    let mut vert_cache: HashMap<(usize, usize, usize), u32> = HashMap::with_capacity(1024);

    let mut any_normals = false;
    let mut face_verts: Vec<u32> = Vec::with_capacity(8);

    for line in src.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut it = line.split_ascii_whitespace();
        let Some(tag) = it.next() else {
            continue;
        };
        match tag {
            "v" => {
                if let (Some(x), Some(y), Some(z)) = (it.next(), it.next(), it.next()) {
                    if let (Ok(x), Ok(y), Ok(z)) =
                        (x.parse::<f32>(), y.parse::<f32>(), z.parse::<f32>())
                    {
                        raw_pos.push([x, y, z]);
                    }
                }
            }
            "vt" => {
                if let (Some(u), Some(v)) = (it.next(), it.next()) {
                    if let (Ok(u), Ok(v)) = (u.parse::<f32>(), v.parse::<f32>()) {
                        raw_uv.push([u, 1.0 - v]);
                    }
                }
            }
            "vn" => {
                if let (Some(x), Some(y), Some(z)) = (it.next(), it.next(), it.next()) {
                    if let (Ok(x), Ok(y), Ok(z)) =
                        (x.parse::<f32>(), y.parse::<f32>(), z.parse::<f32>())
                    {
                        let len = (x * x + y * y + z * z).sqrt().max(1e-6);
                        raw_norm.push([x / len, y / len, z / len]);
                    }
                }
            }
            "f" => {
                face_verts.clear();
                for token in it {
                    let mut parts = token.split('/');
                    let p_str = parts.next().unwrap_or("");
                    let t_str = parts.next().unwrap_or("");
                    let n_str = parts.next().unwrap_or("");

                    let Some(pi) = resolve_obj_index(p_str, raw_pos.len()) else {
                        continue;
                    };
                    let ti = if t_str.is_empty() {
                        usize::MAX
                    } else {
                        resolve_obj_index(t_str, raw_uv.len()).unwrap_or(usize::MAX)
                    };
                    let ni = if n_str.is_empty() {
                        usize::MAX
                    } else {
                        resolve_obj_index(n_str, raw_norm.len()).unwrap_or(usize::MAX)
                    };

                    let key = (pi, ti, ni);
                    let idx = if let Some(&cached) = vert_cache.get(&key) {
                        cached
                    } else {
                        let new_idx = positions.len() as u32;
                        positions.push(raw_pos[pi]);
                        uvs.push(if ti != usize::MAX {
                            raw_uv[ti]
                        } else {
                            [0.0, 0.0]
                        });
                        if ni != usize::MAX {
                            any_normals = true;
                            normals.push(raw_norm[ni]);
                        } else {
                            normals.push([0.0, 1.0, 0.0]);
                        }
                        vert_cache.insert(key, new_idx);
                        new_idx
                    };
                    face_verts.push(idx);
                }

                // Fan-triangulate triangles, quads, and n-gons
                if face_verts.len() >= 3 {
                    let v0 = face_verts[0];
                    for k in 1..(face_verts.len() - 1) {
                        indices.push(v0);
                        indices.push(face_verts[k]);
                        indices.push(face_verts[k + 1]);
                    }
                }
            }
            _ => {}
        }
    }

    if positions.is_empty() || indices.is_empty() {
        return Err(ObjLoadError("OBJ contains no triangles".to_string()));
    }

    if normalize_unit_radius {
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);
        for p in &positions {
            let v = Vec3::from_array(*p);
            min = min.min(v);
            max = max.max(v);
        }
        let center = (min + max) * 0.5;
        let half_extents = (max - min) * 0.5;
        let max_half = half_extents.x.max(half_extents.y).max(half_extents.z);
        let scale = if max_half > 1e-5 { 1.0 / max_half } else { 1.0 };
        for p in &mut positions {
            p[0] = (p[0] - center.x) * scale;
            p[1] = (p[1] - center.y) * scale;
            p[2] = (p[2] - center.z) * scale;
        }
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    if any_normals {
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    }
    mesh.insert_indices(Indices::U32(indices));
    if !any_normals {
        mesh.compute_smooth_normals();
    }
    Ok(mesh)
}

#[derive(Debug, Clone, Copy)]
pub struct ObjBounds {
    pub center: Vec3,
    pub scale: f32,
}

/// Parses an `.obj` with optional shared `(center, scale)` alignment (so slime accessories like
/// `ears_n_tail_LOD0.obj` or `rocks_spine_LOD0.obj` stay anchored to `slime_default.obj`),
/// optional `flip_y_180` (`x = -x, z = -z`), and optional vertical gradient UV mapping (`Top -> Bottom`).
pub fn parse_obj_aligned(
    src: &str,
    align: Option<(Vec3, f32)>,
    vertical_gradient_uv: bool,
    flip_y_180: bool,
) -> Result<(Mesh, ObjBounds), ObjLoadError> {
    let mut mesh = parse_obj_to_mesh(src, false)?;
    let Some(bevy::render::mesh::VertexAttributeValues::Float32x3(pos_attr)) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION).cloned()
    else {
        return Err(ObjLoadError("Missing position attribute".to_string()));
    };

    let mut positions = pos_attr;
    if flip_y_180 {
        for p in &mut positions {
            p[0] = -p[0];
            p[2] = -p[2];
        }
        if let Some(bevy::render::mesh::VertexAttributeValues::Float32x3(norm_attr)) =
            mesh.attribute_mut(Mesh::ATTRIBUTE_NORMAL)
        {
            for n in norm_attr.iter_mut() {
                n[0] = -n[0];
                n[2] = -n[2];
            }
        }
    }

    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    for p in &positions {
        let v = Vec3::from_array(*p);
        min = min.min(v);
        max = max.max(v);
    }

    let (center, scale) = match align {
        Some((c, s)) => (c, s),
        None => {
            let c = (min + max) * 0.5;
            let half = (max - min) * 0.5;
            let max_half = half.x.max(half.y).max(half.z);
            let s = if max_half > 1e-5 { 1.0 / max_half } else { 1.0 };
            (c, s)
        }
    };

    let mut min_y = f32::INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for p in &mut positions {
        p[0] = (p[0] - center.x) * scale;
        p[1] = (p[1] - center.y) * scale;
        p[2] = (p[2] - center.z) * scale;
        min_y = min_y.min(p[1]);
        max_y = max_y.max(p[1]);
    }

    if vertical_gradient_uv {
        let span = (max_y - min_y).max(1e-4);
        let grad_uvs: Vec<[f32; 2]> = positions
            .iter()
            .map(|p| [0.5, (1.0 - (p[1] - min_y) / span).clamp(0.0, 1.0)])
            .collect();
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, grad_uvs);
    }

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.compute_smooth_normals();
    Ok((mesh, ObjBounds { center, scale }))
}

pub fn load_obj_aligned_from_disk(
    path: &std::path::Path,
    align: Option<(Vec3, f32)>,
    vertical_gradient_uv: bool,
    flip_y_180: bool,
) -> Option<(Mesh, ObjBounds)> {
    let text = std::fs::read_to_string(path).ok()?;
    parse_obj_aligned(&text, align, vertical_gradient_uv, flip_y_180).ok()
}

fn ray_triangle_intersect(origin: Vec3, dir: Vec3, v0: Vec3, v1: Vec3, v2: Vec3) -> Option<f32> {
    let e1 = v1 - v0;
    let e2 = v2 - v0;
    let h = dir.cross(e2);
    let a = e1.dot(h);
    if a.abs() < 1e-6 {
        return None;
    }
    let f = 1.0 / a;
    let s = origin - v0;
    let u = f * s.dot(h);
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = s.cross(e1);
    let v = f * dir.dot(q);
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = f * e2.dot(q);
    (t > 0.05).then_some(t)
}

/// Builds a curved face decal patch that conforms closely to `body_mesh` via raycasting from origin,
/// keeping `offset` clearance outside the slime surface so there is zero Z-fighting.
pub fn build_surface_patch_from_mesh(
    body_mesh: &Mesh,
    offset: f32,
    yaw_min: f32,
    yaw_max: f32,
    elev_min: f32,
    elev_max: f32,
    cols: usize,
    rows: usize,
) -> Mesh {
    let pos_slice: &[[f32; 3]] = match body_mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
        Some(bevy::render::mesh::VertexAttributeValues::Float32x3(v)) => v.as_slice(),
        _ => &[],
    };
    let idx_slice: Vec<u32> = match body_mesh.indices() {
        Some(Indices::U32(v)) => v.clone(),
        Some(Indices::U16(v)) => v.iter().map(|&x| x as u32).collect(),
        None => Vec::new(),
    };

    let mut positions = Vec::with_capacity((cols + 1) * (rows + 1));
    let mut normals = Vec::with_capacity((cols + 1) * (rows + 1));
    let mut uvs = Vec::with_capacity((cols + 1) * (rows + 1));
    let mut indices = Vec::with_capacity(cols * rows * 6);

    for r in 0..=rows {
        let vt = r as f32 / rows as f32;
        let elev = elev_max + (elev_min - elev_max) * vt;
        let ce = elev.cos();
        let se = elev.sin();
        for c in 0..=cols {
            let ut = c as f32 / cols as f32;
            let yaw = yaw_max + (yaw_min - yaw_max) * ut;
            let dir = Vec3::new(ce * yaw.sin(), se, ce * yaw.cos()).normalize();

            let mut best_t: Option<f32> = None;
            if !pos_slice.is_empty() && idx_slice.len() >= 3 {
                for tri in idx_slice.chunks_exact(3) {
                    let v0 = Vec3::from_array(pos_slice[tri[0] as usize]);
                    let v1 = Vec3::from_array(pos_slice[tri[1] as usize]);
                    let v2 = Vec3::from_array(pos_slice[tri[2] as usize]);
                    if let Some(t) = ray_triangle_intersect(Vec3::ZERO, dir, v0, v1, v2) {
                        if best_t.map_or(true, |bt| t > bt) {
                            best_t = Some(t);
                        }
                    }
                }
            }

            let dist = best_t.unwrap_or(1.0) + offset;
            let p = dir * dist;
            positions.push([p.x, p.y, p.z]);
            normals.push([dir.x, dir.y, dir.z]);
            uvs.push([ut, vt]);
        }
    }

    let stride = cols + 1;
    for r in 0..rows {
        for c in 0..cols {
            let i0 = (r * stride + c) as u32;
            let i1 = i0 + 1;
            let i2 = ((r + 1) * stride + c) as u32;
            let i3 = i2 + 1;
            indices.extend_from_slice(&[i0, i2, i1, i1, i2, i3]);
        }
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}
