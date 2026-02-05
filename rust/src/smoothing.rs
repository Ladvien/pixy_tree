//! Mesh Smoothing
//!
//! Laplacian smoothing for softening mesh surfaces post-generation.

use godot::prelude::*;
use std::collections::HashMap;

/// Perform Laplacian smoothing on mesh vertices.
///
/// Moves each vertex toward the barycenter of its neighbors,
/// creating smoother, more organic surfaces.
///
/// # Arguments
/// * `vertices` - Mutable slice of vertex positions to smooth
/// * `indices` - Triangle indices defining mesh connectivity
/// * `iterations` - Number of smoothing passes (more = smoother)
/// * `factor` - Blend factor per iteration (0.0-1.0, higher = stronger smoothing)
///
/// # Note
/// Normals should be recalculated after smoothing for correct lighting.
pub fn laplacian_smooth(vertices: &mut [Vector3], indices: &[i32], iterations: u32, factor: f32) {
    if vertices.is_empty() || indices.is_empty() || iterations == 0 {
        return;
    }

    // Build adjacency map: for each vertex, collect its neighbors
    let adjacency = build_adjacency_map(vertices.len(), indices);

    // Perform smoothing iterations
    for _ in 0..iterations {
        smooth_pass(vertices, &adjacency, factor);
    }
}

/// Perform weighted Laplacian smoothing on mesh vertices.
///
/// Like `laplacian_smooth` but with per-vertex weight factors.
/// Useful for applying stronger smoothing at branch junctions
/// while preserving detail on thin branches.
///
/// # Arguments
/// * `vertices` - Mutable slice of vertex positions to smooth
/// * `indices` - Triangle indices defining mesh connectivity
/// * `weights` - Per-vertex weight (0.0 = no smoothing, 1.0 = full smoothing)
/// * `iterations` - Number of smoothing passes
/// * `factor` - Base blend factor per iteration
pub fn laplacian_smooth_weighted(
    vertices: &mut [Vector3],
    indices: &[i32],
    weights: &[f32],
    iterations: u32,
    factor: f32,
) {
    if vertices.is_empty() || indices.is_empty() || iterations == 0 {
        return;
    }
    if weights.len() != vertices.len() {
        // Fallback to uniform if weights don't match
        laplacian_smooth(vertices, indices, iterations, factor);
        return;
    }

    let adjacency = build_adjacency_map(vertices.len(), indices);

    for _ in 0..iterations {
        smooth_pass_weighted(vertices, &adjacency, factor, weights);
    }
}

/// Build a map from vertex index to its neighboring vertex indices.
/// Two vertices are neighbors if they share an edge in any triangle.
///
/// M9 note: C++ builds adjacency from polygon edges (skipping one edge per polygon,
/// typically the diagonal in a quad). Since Rust uses triangulated meshes, we include
/// all triangle edges. This may produce slightly different smoothing results.
/// To match C++ exactly, would need to track original quad topology before triangulation.
fn build_adjacency_map(vertex_count: usize, indices: &[i32]) -> HashMap<usize, Vec<usize>> {
    let mut adjacency: HashMap<usize, Vec<usize>> = HashMap::with_capacity(vertex_count);

    // Initialize empty neighbor lists
    for i in 0..vertex_count {
        adjacency.insert(i, Vec::new());
    }

    // Process triangles to find neighbors
    for triangle in indices.chunks_exact(3) {
        let a = triangle[0] as usize;
        let b = triangle[1] as usize;
        let c = triangle[2] as usize;

        // Each vertex in a triangle is a neighbor of the other two
        add_neighbor(&mut adjacency, a, b);
        add_neighbor(&mut adjacency, a, c);
        add_neighbor(&mut adjacency, b, a);
        add_neighbor(&mut adjacency, b, c);
        add_neighbor(&mut adjacency, c, a);
        add_neighbor(&mut adjacency, c, b);
    }

    adjacency
}

/// Add a neighbor to the adjacency map if not already present
fn add_neighbor(adjacency: &mut HashMap<usize, Vec<usize>>, vertex: usize, neighbor: usize) {
    if let Some(neighbors) = adjacency.get_mut(&vertex) {
        if !neighbors.contains(&neighbor) {
            neighbors.push(neighbor);
        }
    }
}

/// Perform a single smoothing pass
fn smooth_pass(vertices: &mut [Vector3], adjacency: &HashMap<usize, Vec<usize>>, factor: f32) {
    // Calculate new positions (we need to store them separately to avoid
    // using partially-smoothed positions in the same pass)
    let new_positions: Vec<Vector3> = vertices
        .iter()
        .enumerate()
        .map(|(i, &vertex)| {
            if let Some(neighbors) = adjacency.get(&i) {
                if neighbors.len() <= 1 {
                    return vertex;
                }

                // Calculate barycenter of neighbors
                let sum: Vector3 = neighbors
                    .iter()
                    .map(|&n| vertices[n])
                    .fold(Vector3::ZERO, |acc, v| acc + v);
                let barycenter = sum / neighbors.len() as f32;

                // Blend toward barycenter
                vertex.lerp(barycenter, factor)
            } else {
                vertex
            }
        })
        .collect();

    // Apply new positions
    for (i, new_pos) in new_positions.into_iter().enumerate() {
        vertices[i] = new_pos;
    }
}

/// Perform a single weighted smoothing pass
fn smooth_pass_weighted(
    vertices: &mut [Vector3],
    adjacency: &HashMap<usize, Vec<usize>>,
    factor: f32,
    weights: &[f32],
) {
    let new_positions: Vec<Vector3> = vertices
        .iter()
        .enumerate()
        .map(|(i, &vertex)| {
            let weight = weights.get(i).copied().unwrap_or(1.0);
            if weight < 0.001 {
                return vertex; // Skip vertices with negligible weight
            }

            if let Some(neighbors) = adjacency.get(&i) {
                if neighbors.len() <= 1 {
                    return vertex;
                }

                let sum: Vector3 = neighbors
                    .iter()
                    .map(|&n| vertices[n])
                    .fold(Vector3::ZERO, |acc, v| acc + v);
                let barycenter = sum / neighbors.len() as f32;

                // Apply weighted blend
                let effective_factor = factor * weight;
                vertex.lerp(barycenter, effective_factor)
            } else {
                vertex
            }
        })
        .collect();

    for (i, new_pos) in new_positions.into_iter().enumerate() {
        vertices[i] = new_pos;
    }
}

/// Issue C: Average normals for vertices within epsilon distance of each other.
/// This smooths normal seams at mesh junction points (e.g., collar-trunk boundary)
/// without requiring mesh connectivity.
///
/// Uses a spatial hash for efficient neighbor queries.
///
/// # Arguments
/// * `vertices` - Vertex positions
/// * `normals` - Mutable normals to be averaged
/// * `epsilon` - Maximum distance for vertices to be considered coincident
pub fn average_normals_within_epsilon(vertices: &[Vector3], normals: &mut [Vector3], epsilon: f32) {
    if vertices.len() != normals.len() || vertices.is_empty() || epsilon <= 0.0 {
        return;
    }

    let cell_size = epsilon * 2.0;
    let inv_cell_size = 1.0 / cell_size;

    // Build spatial hash: cell -> list of vertex indices
    let mut spatial_hash: HashMap<(i32, i32, i32), Vec<usize>> = HashMap::new();

    for (i, vertex) in vertices.iter().enumerate() {
        let cell = (
            (vertex.x * inv_cell_size).floor() as i32,
            (vertex.y * inv_cell_size).floor() as i32,
            (vertex.z * inv_cell_size).floor() as i32,
        );
        spatial_hash.entry(cell).or_default().push(i);
    }

    // For each vertex, find all neighbors within epsilon and average normals
    let epsilon_sq = epsilon * epsilon;
    let mut averaged_normals = normals.to_vec();

    for (i, vertex) in vertices.iter().enumerate() {
        let cell = (
            (vertex.x * inv_cell_size).floor() as i32,
            (vertex.y * inv_cell_size).floor() as i32,
            (vertex.z * inv_cell_size).floor() as i32,
        );

        // Collect normals from neighbors in adjacent cells
        let mut sum = normals[i];
        let mut count = 1;

        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    let neighbor_cell = (cell.0 + dx, cell.1 + dy, cell.2 + dz);
                    if let Some(indices) = spatial_hash.get(&neighbor_cell) {
                        for &j in indices {
                            if j != i {
                                let dist_sq = (vertices[j] - *vertex).length_squared();
                                if dist_sq < epsilon_sq {
                                    sum += normals[j];
                                    count += 1;
                                }
                            }
                        }
                    }
                }
            }
        }

        if count > 1 {
            // Only update if we found neighbors
            let avg = sum / count as f32;
            if avg.length_squared() > 0.0001 {
                averaged_normals[i] = avg.normalized();
            }
        }
    }

    // Copy back averaged normals
    normals.copy_from_slice(&averaged_normals);
}

/// Recalculate normals after smoothing.
/// Computes smooth normals by averaging face normals at each vertex.
pub fn recalculate_normals(vertices: &[Vector3], indices: &[i32], normals: &mut [Vector3]) {
    if vertices.len() != normals.len() {
        return;
    }

    // Zero out all normals
    for normal in normals.iter_mut() {
        *normal = Vector3::ZERO;
    }

    // Accumulate face normals for each vertex
    for triangle in indices.chunks_exact(3) {
        let a = triangle[0] as usize;
        let b = triangle[1] as usize;
        let c = triangle[2] as usize;

        let v0 = vertices[a];
        let v1 = vertices[b];
        let v2 = vertices[c];

        // Calculate face normal
        let edge1 = v1 - v0;
        let edge2 = v2 - v0;
        let face_normal = edge1.cross(edge2);

        // Add face normal to each vertex (weighted by face area implicitly)
        normals[a] += face_normal;
        normals[b] += face_normal;
        normals[c] += face_normal;
    }

    // Normalize all normals
    for normal in normals.iter_mut() {
        if normal.length_squared() > 0.0001 {
            *normal = normal.normalized();
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
        let adj = build_adjacency_map(3, &indices);

        assert!(adj.get(&0).unwrap().contains(&1));
        assert!(adj.get(&0).unwrap().contains(&2));
        assert!(adj.get(&1).unwrap().contains(&0));
        assert!(adj.get(&1).unwrap().contains(&2));
        assert!(adj.get(&2).unwrap().contains(&0));
        assert!(adj.get(&2).unwrap().contains(&1));
    }

    #[test]
    fn test_laplacian_smooth() {
        // A simple quad (2 triangles) with a raised center vertex
        let mut vertices = vec![
            Vector3::new(0.0, 0.0, 0.0), // 0: corner
            Vector3::new(1.0, 0.0, 0.0), // 1: corner
            Vector3::new(1.0, 0.0, 1.0), // 2: corner
            Vector3::new(0.0, 0.0, 1.0), // 3: corner
            Vector3::new(0.5, 1.0, 0.5), // 4: raised center
        ];

        // Two triangles sharing the center vertex
        let indices = vec![
            0, 1, 4, // tri 1
            1, 2, 4, // tri 2
            2, 3, 4, // tri 3
            3, 0, 4, // tri 4
        ];

        let initial_center_y = vertices[4].y;

        // Apply smoothing
        laplacian_smooth(&mut vertices, &indices, 1, 0.5);

        // The center vertex should move toward its neighbors (downward)
        assert!(vertices[4].y < initial_center_y);
    }

    #[test]
    fn test_no_smoothing_with_zero_iterations() {
        let mut vertices = vec![
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(1.0, 1.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
        ];
        let original = vertices.clone();
        let indices = vec![0, 1, 2];

        laplacian_smooth(&mut vertices, &indices, 0, 0.5);

        for (v, o) in vertices.iter().zip(original.iter()) {
            assert_eq!(v, o);
        }
    }

    #[test]
    fn test_recalculate_normals() {
        // Simple upward-facing triangle (counter-clockwise when viewed from above)
        let vertices = vec![
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
            Vector3::new(1.0, 0.0, 0.0),
        ];
        let indices = vec![0, 1, 2];
        let mut normals = vec![Vector3::ZERO; 3];

        recalculate_normals(&vertices, &indices, &mut normals);

        // All normals should point up (positive Y)
        for normal in &normals {
            assert!(normal.y > 0.5);
        }
    }
}
