use godot::prelude::*;
use std::f32::consts::TAU;

use crate::branch::MeshData;

/// Linear interpolation for f32
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Get a perpendicular vector to the given direction
fn get_perpendicular(dir: Vector3) -> Vector3 {
    let up = if dir.y.abs() < 0.9 {
        Vector3::UP
    } else {
        Vector3::RIGHT
    };
    dir.cross(up).normalized()
}

/// Generate a collar mesh that smoothly connects trunk surface to branch base.
/// This creates a tapered cone-like transition that bridges the gap between
/// the trunk and branch, eliminating visible seams.
///
/// # Arguments
/// * `branch_start` - Branch attachment point on trunk surface
/// * `branch_direction` - Branch growth direction (normalized)
/// * `trunk_radius` - Trunk radius at this height
/// * `branch_radius` - Branch base radius
/// * `collar_length` - How far collar extends along branch direction
/// * `radial_segments` - Segments around circumference
pub fn generate_branch_collar(
    branch_start: Vector3,
    branch_direction: Vector3,
    _trunk_radius: f32,
    branch_radius: f32,
    collar_length: f32,
    radial_segments: i32,
) -> MeshData {
    let mut mesh = MeshData::new();

    let segments = radial_segments.max(3) as usize;
    let rings = 3usize; // Base ring on trunk, middle blend, end at branch start

    // Build rotation basis from branch direction
    let right = get_perpendicular(branch_direction);
    let forward = branch_direction.cross(right);

    // Calculate the collar's starting position (slightly embedded in trunk)
    // and ending position (at branch start)
    let collar_start = branch_start - branch_direction * collar_length * 0.1;
    let collar_end = branch_start + branch_direction * collar_length * 0.5;

    // The collar starts with a wider "flare" at the trunk and tapers to branch radius
    // Start radius is larger to blend with trunk surface
    let start_radius = branch_radius * 1.3;
    let end_radius = branch_radius;

    // Generate rings of vertices
    for ring in 0..rings {
        let t = ring as f32 / (rings - 1) as f32;

        // Position along collar (lerp from start to end)
        let pos = Vector3::new(
            lerp(collar_start.x, collar_end.x, t),
            lerp(collar_start.y, collar_end.y, t),
            lerp(collar_start.z, collar_end.z, t),
        );

        // Radius tapers from start_radius to end_radius
        // Use smooth interpolation for natural look
        let smooth_t = t * t * (3.0 - 2.0 * t); // Smoothstep
        let radius = lerp(start_radius, end_radius, smooth_t);

        let v = t;

        for seg in 0..=segments {
            let angle = (seg as f32 / segments as f32) * TAU;

            let local_x = angle.cos() * radius;
            let local_z = angle.sin() * radius;

            // Transform to world space using our basis
            let offset = right * local_x + forward * local_z;
            let vertex = pos + offset;

            // Normal points outward in the local frame
            let normal = (right * angle.cos() + forward * angle.sin()).normalized();

            mesh.vertices.push(vertex);
            mesh.normals.push(normal);
            mesh.uvs.push(Vector2::new(seg as f32 / segments as f32, v));
        }
    }

    // Generate indices for collar sides (connect rings with triangles)
    let verts_per_ring = segments + 1;
    for ring in 0..(rings - 1) {
        for seg in 0..segments {
            let current = (ring * verts_per_ring + seg) as i32;
            let next = (ring * verts_per_ring + seg + 1) as i32;
            let above = ((ring + 1) * verts_per_ring + seg) as i32;
            let above_next = ((ring + 1) * verts_per_ring + seg + 1) as i32;

            // Two triangles per quad (counter-clockwise winding for front faces)
            mesh.indices.extend_from_slice(&[current, next, above]);
            mesh.indices.extend_from_slice(&[next, above_next, above]);
        }
    }

    mesh
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_branch_collar() {
        let collar = generate_branch_collar(
            Vector3::new(0.5, 2.0, 0.0),
            Vector3::new(1.0, 0.5, 0.0).normalized(),
            0.5,
            0.1,
            0.15,
            6,
        );

        // Should have vertices
        assert!(!collar.vertices.is_empty());
        // Should have 3 rings * 7 vertices each = 21 vertices
        assert_eq!(collar.vertices.len(), 21);
        // Should have 2 ring connections * 6 segments * 2 triangles * 3 indices = 72 indices
        assert_eq!(collar.indices.len(), 72);
    }

    #[test]
    fn test_collar_has_matching_normals_and_uvs() {
        let collar = generate_branch_collar(
            Vector3::new(0.3, 1.5, 0.2),
            Vector3::UP,
            0.4,
            0.08,
            0.12,
            8,
        );

        assert_eq!(collar.vertices.len(), collar.normals.len());
        assert_eq!(collar.vertices.len(), collar.uvs.len());
    }
}
