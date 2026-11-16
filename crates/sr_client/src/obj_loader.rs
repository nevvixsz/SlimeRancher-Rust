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
