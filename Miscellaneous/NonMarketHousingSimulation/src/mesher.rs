//! Geometry to CPU mesh data conversion (triangulation and wall extrusion).

use bevy::prelude::*;
use geo_types::{Coord, Geometry, LineString, MultiPolygon, Polygon};

// Raw mesh data

#[derive(Debug, Clone, Default)]
pub struct MeshData {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub colors: Vec<[f32; 4]>,
    pub indices: Vec<u32>,
}

impl MeshData {
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    pub fn merge(&mut self, other: MeshData) {
        if other.is_empty() {
            return;
        }
        let offset = self.positions.len() as u32;
        self.positions.extend(other.positions);
        self.normals.extend(other.normals);
        self.colors.extend(other.colors);
        self.indices
            .extend(other.indices.iter().map(|i| i + offset));
    }

    pub fn into_bevy_mesh(self) -> Mesh {
        use bevy::asset::RenderAssetUsages;
        use bevy::mesh::{Indices, PrimitiveTopology};

        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        if !self.colors.is_empty() {
            mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, self.colors);
        }
        mesh.insert_indices(Indices::U32(self.indices));
        mesh
    }
}

// Coordinate helpers

pub const TILE_HALF: f32 = 2048.0;

#[inline]
pub fn mvt_to_xz(x: f32, y: f32, scale: f32) -> (f32, f32) {
    ((x - TILE_HALF) * scale, (y - TILE_HALF) * scale)
}

#[inline]
pub fn mvt_to_xy(x: f32, y: f32, scale: f32) -> (f32, f32) {
    ((x - TILE_HALF) * scale, -(y - TILE_HALF) * scale)
}

// Main geometry dispatch

pub fn geometry_to_mesh(
    geom: &Geometry<f32>,
    scale: f32,
    base_height: f32,
    extrude_top: Option<f32>,
    line_width: f32,
    color: [f32; 4],
    point_out: &mut Vec<[f32; 3]>,
) -> Option<MeshData> {
    match geom {
        Geometry::Point(p) => {
            let (x, z) = mvt_to_xz(p.x(), p.y(), scale);
            point_out.push([x, base_height, z]);
            None
        }
        Geometry::MultiPoint(mp) => {
            for p in &mp.0 {
                let (x, z) = mvt_to_xz(p.x(), p.y(), scale);
                point_out.push([x, base_height, z]);
            }
            None
        }
        Geometry::LineString(ls) => Some(build_line(ls, scale, base_height, line_width, color)),
        Geometry::MultiLineString(mls) => {
            let mut out = MeshData::default();
            for ls in &mls.0 {
                out.merge(build_line(ls, scale, base_height, line_width, color));
            }
            Some(out).filter(|m| !m.is_empty())
        }
        Geometry::Polygon(poly) => {
            let m = if let Some(top) = extrude_top {
                build_extruded_polygon(poly, scale, base_height, top, color)
            } else {
                build_flat_polygon(poly, scale, base_height, color)
            };
            Some(m).filter(|m| !m.is_empty())
        }
        Geometry::MultiPolygon(mp) => {
            let m = if let Some(top) = extrude_top {
                build_extruded_multipolygon(mp, scale, base_height, top, color)
            } else {
                build_flat_multipolygon(mp, scale, base_height, color)
            };
            Some(m).filter(|m| !m.is_empty())
        }
        _ => None,
    }
}

pub fn geometry_to_mesh_2d(
    geom: &Geometry<f32>,
    scale: f32,
    z_depth: f32,
    line_width: f32,
    color: [f32; 4],
    point_out: &mut Vec<[f32; 3]>,
) -> Option<MeshData> {
    match geom {
        Geometry::Point(p) => {
            let (x, y) = mvt_to_xy(p.x(), p.y(), scale);
            point_out.push([x, y, z_depth]);
            None
        }
        Geometry::MultiPoint(mp) => {
            for p in &mp.0 {
                let (x, y) = mvt_to_xy(p.x(), p.y(), scale);
                point_out.push([x, y, z_depth]);
            }
            None
        }
        Geometry::LineString(ls) => Some(build_line_2d(ls, scale, z_depth, line_width, color)),
        Geometry::MultiLineString(mls) => {
            let mut out = MeshData::default();
            for ls in &mls.0 {
                out.merge(build_line_2d(ls, scale, z_depth, line_width, color));
            }
            Some(out).filter(|m| !m.is_empty())
        }
        Geometry::Polygon(poly) => Some(build_flat_polygon_2d(poly, scale, z_depth, color)),
        Geometry::MultiPolygon(mp) => Some(build_flat_multipolygon_2d(mp, scale, z_depth, color)),
        _ => None,
    }
}

// Line, technically a quad

pub fn build_line(
    ls: &LineString<f32>,
    scale: f32,
    height: f32,
    width: f32,
    color: [f32; 4],
) -> MeshData {
    let coords = &ls.0;
    if coords.len() < 2 {
        return MeshData::default();
    }
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();
    let up = [0.0f32, 1.0, 0.0];
    for window in coords.windows(2) {
        let (ax, az) = mvt_to_xz(window[0].x, window[0].y, scale);
        let (bx, bz) = mvt_to_xz(window[1].x, window[1].y, scale);
        let dx = bx - ax;
        let dz = bz - az;
        let len = (dx * dx + dz * dz).sqrt();
        if len < 1e-6 {
            continue;
        }
        let px = (-dz / len) * width * 0.5;
        let pz = (dx / len) * width * 0.5;
        let base = positions.len() as u32;
        positions.extend_from_slice(&[
            [ax - px, height, az - pz],
            [ax + px, height, az + pz],
            [bx + px, height, bz + pz],
            [bx - px, height, bz - pz],
        ]);
        for _ in 0..4 {
            normals.push(up);
            colors.push(color);
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    MeshData {
        positions,
        normals,
        colors,
        indices,
    }
}

pub fn build_flat_polygon(
    poly: &Polygon<f32>,
    scale: f32,
    height: f32,
    color: [f32; 4],
) -> MeshData {
    triangulate_ring(poly, scale, height, [0.0, 1.0, 0.0], color)
}

pub fn build_flat_multipolygon(
    mp: &MultiPolygon<f32>,
    scale: f32,
    height: f32,
    color: [f32; 4],
) -> MeshData {
    let mut out = MeshData::default();
    for poly in &mp.0 {
        out.merge(build_flat_polygon(poly, scale, height, color));
    }
    out
}

pub fn build_line_2d(
    ls: &LineString<f32>,
    scale: f32,
    z_depth: f32,
    width: f32,
    color: [f32; 4],
) -> MeshData {
    let coords = &ls.0;
    if coords.len() < 2 {
        return MeshData::default();
    }
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();
    let up = [0.0f32, 0.0, 1.0];
    for window in coords.windows(2) {
        let (ax, ay) = mvt_to_xy(window[0].x, window[0].y, scale);
        let (bx, by) = mvt_to_xy(window[1].x, window[1].y, scale);
        let dx = bx - ax;
        let dy = by - ay;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1e-6 {
            continue;
        }
        let px = (-dy / len) * width * 0.5;
        let py = (dx / len) * width * 0.5;
        let base = positions.len() as u32;
        positions.extend_from_slice(&[
            [ax - px, ay - py, z_depth],
            [ax + px, ay + py, z_depth],
            [bx + px, by + py, z_depth],
            [bx - px, by - py, z_depth],
        ]);
        for _ in 0..4 {
            normals.push(up);
            colors.push(color);
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    MeshData {
        positions,
        normals,
        colors,
        indices,
    }
}

pub fn build_flat_polygon_2d(
    poly: &Polygon<f32>,
    scale: f32,
    z_depth: f32,
    color: [f32; 4],
) -> MeshData {
    triangulate_ring_2d(poly, scale, z_depth, [0.0, 0.0, 1.0], color)
}

pub fn build_flat_multipolygon_2d(
    mp: &MultiPolygon<f32>,
    scale: f32,
    z_depth: f32,
    color: [f32; 4],
) -> MeshData {
    let mut out = MeshData::default();
    for poly in &mp.0 {
        out.merge(build_flat_polygon_2d(poly, scale, z_depth, color));
    }
    out
}

// Extruded polygon for buildings

pub fn build_extruded_polygon(
    poly: &Polygon<f32>,
    scale: f32,
    min_h: f32,
    max_h: f32,
    color: [f32; 4],
) -> MeshData {
    let mut out = MeshData::default();
    out.merge(triangulate_ring(poly, scale, max_h, [0.0, 1.0, 0.0], color));
    let ext: Vec<&Coord<f32>> = poly.exterior().0.iter().collect();
    out.merge(build_walls(&ext, scale, min_h, max_h, color));
    for hole in poly.interiors() {
        let hc: Vec<&Coord<f32>> = hole.0.iter().collect();
        out.merge(build_walls(&hc, scale, min_h, max_h, color));
    }
    out
}

pub fn build_extruded_multipolygon(
    mp: &MultiPolygon<f32>,
    scale: f32,
    min_h: f32,
    max_h: f32,
    color: [f32; 4],
) -> MeshData {
    let mut out = MeshData::default();
    for poly in &mp.0 {
        out.merge(build_extruded_polygon(poly, scale, min_h, max_h, color));
    }
    out
}

// Internals

fn triangulate_ring(
    poly: &Polygon<f32>,
    scale: f32,
    height: f32,
    normal: [f32; 3],
    color: [f32; 4],
) -> MeshData {
    let mut flat: Vec<f64> = Vec::new();
    let mut hole_indices: Vec<usize> = Vec::new();
    for c in &poly.exterior().0 {
        flat.push(c.x as f64);
        flat.push(c.y as f64);
    }
    for hole in poly.interiors() {
        hole_indices.push(flat.len() / 2);
        for c in &hole.0 {
            flat.push(c.x as f64);
            flat.push(c.y as f64);
        }
    }
    let n_verts = flat.len() / 2;
    if n_verts < 3 {
        return MeshData::default();
    }
    let tri_idx = match earcutr::earcut(&flat, &hole_indices, 2) {
        Ok(v) if !v.is_empty() => v,
        _ => return MeshData::default(),
    };
    let mut positions = Vec::with_capacity(n_verts);
    let mut normals = Vec::with_capacity(n_verts);
    let mut colors = Vec::with_capacity(n_verts);
    for i in 0..n_verts {
        let (wx, wz) = mvt_to_xz(flat[i * 2] as f32, flat[i * 2 + 1] as f32, scale);
        positions.push([wx, height, wz]);
        normals.push(normal);
        colors.push(color);
    }
    let mut indices: Vec<u32> = tri_idx.iter().map(|&i| i as u32).collect();
    // MVT exteriors are CW; earcut produces CW triangles. Reverse to get CCW front faces.
    indices.reverse();
    MeshData {
        positions,
        normals,
        colors,
        indices,
    }
}

fn triangulate_ring_2d(
    poly: &Polygon<f32>,
    scale: f32,
    z_depth: f32,
    normal: [f32; 3],
    color: [f32; 4],
) -> MeshData {
    let mut flat: Vec<f64> = Vec::new();
    let mut hole_indices: Vec<usize> = Vec::new();
    for c in &poly.exterior().0 {
        flat.push(c.x as f64);
        flat.push(c.y as f64);
    }
    for hole in poly.interiors() {
        hole_indices.push(flat.len() / 2);
        for c in &hole.0 {
            flat.push(c.x as f64);
            flat.push(c.y as f64);
        }
    }
    let n_verts = flat.len() / 2;
    if n_verts < 3 {
        return MeshData::default();
    }
    let tri_idx = match earcutr::earcut(&flat, &hole_indices, 2) {
        Ok(v) if !v.is_empty() => v,
        _ => return MeshData::default(),
    };
    let mut positions = Vec::with_capacity(n_verts);
    let mut normals = Vec::with_capacity(n_verts);
    let mut colors = Vec::with_capacity(n_verts);
    for i in 0..n_verts {
        let (wx, wy) = mvt_to_xy(flat[i * 2] as f32, flat[i * 2 + 1] as f32, scale);
        positions.push([wx, wy, z_depth]);
        normals.push(normal);
        colors.push(color);
    }
    let mut indices: Vec<u32> = tri_idx.iter().map(|&i| i as u32).collect();
    indices.reverse();
    MeshData {
        positions,
        normals,
        colors,
        indices,
    }
}

fn build_walls(
    coords: &[&Coord<f32>],
    scale: f32,
    min_h: f32,
    max_h: f32,
    color: [f32; 4],
) -> MeshData {
    let n = coords.len();
    if n < 2 {
        return MeshData::default();
    }
    let close = n > 2
        && (coords[0].x - coords[n - 1].x).abs() < 1e-4
        && (coords[0].y - coords[n - 1].y).abs() < 1e-4;
    let end = if close { n - 1 } else { n };
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();
    for i in 0..end {
        let a = coords[i];
        let b = coords[(i + 1) % end];
        let (ax, az) = mvt_to_xz(a.x, a.y, scale);
        let (bx, bz) = mvt_to_xz(b.x, b.y, scale);
        let dx = bx - ax;
        let dz = bz - az;
        let len = (dx * dx + dz * dz).sqrt();
        let (nx, nz) = if len > 1e-6 {
            (dz / len, -dx / len)
        } else {
            (0.0, 1.0)
        };
        let base = positions.len() as u32;
        positions.extend_from_slice(&[
            [ax, min_h, az],
            [bx, min_h, bz],
            [bx, max_h, bz],
            [ax, max_h, az],
        ]);
        let wn = [nx, 0.0, nz];
        for _ in 0..4 {
            normals.push(wn);
            colors.push(color);
        }
        indices.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
    }
    MeshData {
        positions,
        normals,
        colors,
        indices,
    }
}
