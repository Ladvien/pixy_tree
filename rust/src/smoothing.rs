/// Mesh smoothing utilities for manifold topology
///
/// Provides Laplacian smoothing for junction vertices to create smooth transitions
/// between trunk and branch geometry.
use godot::prelude::*;
use std::collections::{HashMap, HashSet};

/// Apply Laplacian smoothing to a set of junction vertices in a mesh.
///
/// This moves each vertex toward the average position of its neighbors,
/// creating smoother transitions at junctions.
///
/// # Arguments
/// * `vertices` - Mutable slice of all vertex positions
/// * `normals` - Mutable slice of all vertex normals (will be recalculated)
/// * `indices` - Triangle indices defining mesh connectivity
/// * `junction_indices` - Set of vertex indices to smooth
/// * `iterations` - Number of smoothing iterations (1-3 recommended)
/// * `factor` - Smoothing strength (0.0-1.0, 0.3-0.5 recommended)
pub fn laplacian_smooth(
    vertices: &mut [Vector3],
    normals: &mut [Vector3],
    indices: &[i32],
    junction_indices: &HashSet<u32>,
    iterations: u32,
    factor: f32,
) {
    if junction_indices.is_empty() || vertices.is_empty() {
        return;
    }

    // Build adjacency map: for each vertex, which other vertices are neighbors
    let adjacency = build_adjacency_map(indices, vertices.len());

    // Perform smoothing iterations
    for _ in 0..iterations {
        // Collect new positions (we don't modify in-place during iteration)
        let mut new_positions: HashMap<u32, Vector3> = HashMap::new();

        for &idx in junction_indices {
            if let Some(neighbors) = adjacency.get(&idx) {
                if neighbors.is_empty() {
                    continue;
                }

                // Calculate barycenter of neighbors
                let mut barycenter = Vector3::ZERO;
                for &neighbor_idx in neighbors {
                    barycenter += vertices[neighbor_idx as usize];
                }
                barycenter /= neighbors.len() as f32;

                // Interpolate toward barycenter
                let current = vertices[idx as usize];
                let smoothed = current.lerp(barycenter, factor);
                new_positions.insert(idx, smoothed);
            }
        }

        // Apply new positions
        for (idx, pos) in new_positions {
            vertices[idx as usize] = pos;
        }
    }

    // Recalculate normals for smoothed vertices
    recalculate_normals_for_vertices(vertices, normals, indices, junction_indices);
}

/// Build an adjacency map from triangle indices.
/// Maps each vertex index to its neighboring vertex indices.
fn build_adjacency_map(indices: &[i32], vertex_count: usize) -> HashMap<u32, HashSet<u32>> {
    let mut adjacency: HashMap<u32, HashSet<u32>> = HashMap::new();

    // Initialize empty neighbor sets
    for i in 0..vertex_count {
        adjacency.insert(i as u32, HashSet::new());
    }

    // Process triangles
    for tri in indices.chunks(3) {
        if tri.len() < 3 {
            continue;
        }

        let i0 = tri[0] as u32;
        let i1 = tri[1] as u32;
        let i2 = tri[2] as u32;

        // Each vertex is adjacent to the other two in the triangle
        if let Some(set) = adjacency.get_mut(&i0) {
            set.insert(i1);
            set.insert(i2);
        }
        if let Some(set) = adjacency.get_mut(&i1) {
            set.insert(i0);
            set.insert(i2);
        }
        if let Some(set) = adjacency.get_mut(&i2) {
            set.insert(i0);
            set.insert(i1);
        }
    }

    adjacency
}

/// Recalculate normals for a set of vertices based on adjacent face normals.
fn recalculate_normals_for_vertices(
    vertices: &[Vector3],
    normals: &mut [Vector3],
    indices: &[i32],
    vertex_indices: &HashSet<u32>,
) {
    // Map vertex index to accumulated normal
    let mut normal_accum: HashMap<u32, Vector3> = HashMap::new();
    let mut normal_count: HashMap<u32, u32> = HashMap::new();

    for idx in vertex_indices {
        normal_accum.insert(*idx, Vector3::ZERO);
        normal_count.insert(*idx, 0);
    }

    // Accumulate face normals
    for tri in indices.chunks(3) {
        if tri.len() < 3 {
            continue;
        }

        let i0 = tri[0] as u32;
        let i1 = tri[1] as u32;
        let i2 = tri[2] as u32;

        // Check if any vertex in this triangle is being smoothed
        let affects_smoothed = vertex_indices.contains(&i0)
            || vertex_indices.contains(&i1)
            || vertex_indices.contains(&i2);

        if !affects_smoothed {
            continue;
        }

        // Calculate face normal
        let v0 = vertices[i0 as usize];
        let v1 = vertices[i1 as usize];
        let v2 = vertices[i2 as usize];

        let edge1 = v1 - v0;
        let edge2 = v2 - v0;
        let face_normal = edge1.cross(edge2).normalized();

        // Skip degenerate triangles
        if face_normal.length_squared() < 0.0001 {
            continue;
        }

        // Accumulate for affected vertices
        for idx in [i0, i1, i2] {
            if vertex_indices.contains(&idx) {
                if let Some(accum) = normal_accum.get_mut(&idx) {
                    *accum += face_normal;
                }
                if let Some(count) = normal_count.get_mut(&idx) {
                    *count += 1;
                }
            }
        }
    }

    // Apply averaged normals
    for (&idx, accum) in &normal_accum {
        if let Some(&count) = normal_count.get(&idx) {
            if count > 0 {
                let averaged = (*accum / count as f32).normalized();
                if averaged.length_squared() > 0.0001 {
                    normals[idx as usize] = averaged;
                }
            }
        }
    }
}

/// Identify junction vertices where trunk and branch geometry meet.
/// These are vertices shared between trunk hole boundary and branch transition.
///
/// Returns a set of vertex indices that should be smoothed.
#[allow(dead_code)]
pub fn identify_junction_vertices(
    hole_boundary_indices: &[u32],
    transition_ring_indices: &[u32],
) -> HashSet<u32> {
    let mut junction: HashSet<u32> = HashSet::new();

    // All hole boundary vertices are junction vertices
    junction.extend(hole_boundary_indices.iter().copied());

    // All transition ring vertices are junction vertices
    junction.extend(transition_ring_indices.iter().copied());

    junction
}

/// Blend normals at junction vertices between trunk and branch normals.
///
/// # Arguments
/// * `normals` - Mutable slice of all normals
/// * `junction_indices` - Vertices at the junction
/// * `trunk_direction` - Outward direction from trunk at junction
/// * `branch_direction` - Direction of branch growth
/// * `blend_factor` - 0.0 = pure trunk normal, 1.0 = pure branch normal
#[allow(dead_code)]
pub fn blend_junction_normals(
    normals: &mut [Vector3],
    junction_indices: &HashSet<u32>,
    trunk_direction: Vector3,
    branch_direction: Vector3,
    blend_factor: f32,
) {
    let blend = blend_factor.clamp(0.0, 1.0);

    for &idx in junction_indices {
        if (idx as usize) < normals.len() {
            let current = normals[idx as usize];

            // Project current normal onto trunk vs branch influence
            let trunk_influence = trunk_direction.normalized();
            let branch_influence = branch_direction.normalized();

            // Blend between trunk-aligned and branch-aligned
            let blended = trunk_influence.lerp(branch_influence, blend);

            // Blend with original normal
            let final_normal = current.lerp(blended, 0.5).normalized();

            if final_normal.length_squared() > 0.0001 {
                normals[idx as usize] = final_normal;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_adjacency_map() {
        // Simple triangle
        let indices = vec![0, 1, 2];
        let adjacency = build_adjacency_map(&indices, 3);

        assert!(adjacency.get(&0).unwrap().contains(&1));
        assert!(adjacency.get(&0).unwrap().contains(&2));
        assert!(adjacency.get(&1).unwrap().contains(&0));
        assert!(adjacency.get(&1).unwrap().contains(&2));
        assert!(adjacency.get(&2).unwrap().contains(&0));
        assert!(adjacency.get(&2).unwrap().contains(&1));
    }

    #[test]
    fn test_identify_junction_vertices() {
        let hole_boundary = vec![0, 1, 2, 3];
        let transition_ring = vec![4, 5, 6, 7];

        let junction = identify_junction_vertices(&hole_boundary, &transition_ring);

        assert_eq!(junction.len(), 8);
        assert!(junction.contains(&0));
        assert!(junction.contains(&4));
    }

    #[test]
    fn test_laplacian_smooth_preserves_non_junction() {
        // Create a simple mesh where we only smooth one vertex
        let mut vertices = vec![
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
        ];
        let mut normals = vec![Vector3::UP, Vector3::UP, Vector3::UP];
        let indices = vec![0, 1, 2];

        // Only smooth vertex 0
        let mut junction = HashSet::new();
        junction.insert(0);

        // Store original positions of non-junction vertices
        let orig_v1 = vertices[1];
        let orig_v2 = vertices[2];

        laplacian_smooth(&mut vertices, &mut normals, &indices, &junction, 1, 0.5);

        // Non-junction vertices should be unchanged
        assert_eq!(vertices[1], orig_v1);
        assert_eq!(vertices[2], orig_v2);
    }
}
