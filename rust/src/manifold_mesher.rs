//! Manifold Mesher — watertight mesh generation with proper branch junctions.
//!
//! Ports the C++ ManifoldMesher from modular_tree. Instead of generating separate
//! cylinders per branch segment, this mesher creates a single continuous mesh with
//! proper topology at branch attachment points (no overlapping geometry).
//!
//! The key data structures are:
//! - `BranchNode`: a tree graph node with children (continuation + side branches)
//! - `BranchNodeChild`: wraps a `BranchNode` with `position_in_parent`
//! - The mesher traverses this tree recursively, placing radial vertex circles
//!   and cutting holes where side branches attach.

use godot::prelude::*;
use std::f32::consts::TAU;

use crate::branch::{BranchSegment, MeshData};

// ════════════════════════════════════════════════════════════
// Tree Graph Types
// ════════════════════════════════════════════════════════════

/// A node in the branch tree graph for manifold meshing.
/// `children[0]` is always the continuation; `children[1..]` are side branches.
#[derive(Clone, Debug)]
pub struct BranchNode {
    pub direction: Vector3,
    pub tangent: Vector3,
    pub length: f32,
    pub radius: f32,
    #[allow(dead_code)]
    pub depth: u8,
    #[allow(dead_code)]
    pub height_ratio: f32,
    pub children: Vec<BranchNodeChild>,
}

/// A child attachment on a parent BranchNode.
#[derive(Clone, Debug)]
pub struct BranchNodeChild {
    pub node: BranchNode,
    /// Where along the parent this child attaches (0.0-1.0)
    pub position_in_parent: f32,
}

impl BranchNode {
    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }
}

// ════════════════════════════════════════════════════════════
// Internal Mesher Types
// ════════════════════════════════════════════════════════════

#[derive(Clone, Debug)]
struct CircleDesignator {
    vertex_index: usize,
    #[allow(dead_code)] // Used for UV offset tracking, read indirectly via struct patterns
    uv_index: usize,
    radial_n: usize,
}

#[derive(Clone, Copy, Debug)]
struct IndexRange {
    min_index: i32,
    max_index: i32,
}

/// Pivot Painter 2.0 per-vertex attributes
#[derive(Clone, Debug)]
pub struct PivotPainterContext {
    pub stem_id: i32,
    pub hierarchy_depth: i32,
    pub pivot_position: Vector3,
    pub branch_extent: f32,
}

/// Extended mesh data that also collects pivot painter attributes
#[derive(Default)]
pub struct ManifoldMeshData {
    pub mesh: MeshData,
    /// Per-vertex: [stem_id, hierarchy_depth, branch_extent, 0.0]
    pub custom0: Vec<[f32; 4]>,
    /// Per-vertex: [pivot_x, pivot_y, pivot_z, 0.0]
    pub custom1: Vec<[f32; 4]>,
    /// Per-vertex branch direction (for direction-based shaders)
    pub directions: Vec<Vector3>,
    /// L4: Per-vertex radius (for radius-based shaders)
    pub radii: Vec<f32>,
    /// Separate UV tracking (ManifoldMesher generates UVs separately from vertices)
    uv_list: Vec<Vector2>,
}

impl ManifoldMeshData {
    fn new() -> Self {
        Self::default()
    }

    /// Add a vertex and return its index
    fn add_vertex(&mut self, position: Vector3) -> usize {
        let idx = self.mesh.vertices.len();
        self.mesh.vertices.push(position);
        self.mesh.normals.push(Vector3::ZERO); // Will be recalculated
        self.mesh.smooth_weights.push(1.0);
        self.custom0.push([0.0; 4]);
        self.custom1.push([0.0; 4]);
        self.directions.push(Vector3::UP);
        self.radii.push(0.0); // L4: Default radius, will be set by caller
        idx
    }

    /// Add a vertex with radius and return its index (L4)
    fn add_vertex_with_radius(&mut self, position: Vector3, radius: f32) -> usize {
        let idx = self.add_vertex(position);
        self.radii[idx] = radius;
        idx
    }

    /// Add a quad (4 vertex indices) as two triangles
    fn add_quad(&mut self, v0: usize, v1: usize, v2: usize, v3: usize) {
        // Quad v0-v1-v2-v3 → triangles v0-v1-v2 and v0-v2-v3
        self.mesh.indices.push(v0 as i32);
        self.mesh.indices.push(v1 as i32);
        self.mesh.indices.push(v2 as i32);

        self.mesh.indices.push(v0 as i32);
        self.mesh.indices.push(v2 as i32);
        self.mesh.indices.push(v3 as i32);
    }
}

// ════════════════════════════════════════════════════════════
// Segment-to-Tree Conversion
// ════════════════════════════════════════════════════════════

/// Get a perpendicular vector to the given direction
fn get_perpendicular(dir: Vector3) -> Vector3 {
    let up = if dir.y.abs() < 0.9 {
        Vector3::UP
    } else {
        Vector3::RIGHT
    };
    dir.cross(up).normalized()
}

/// Convert a flat list of BranchSegments into a tree of BranchNodes.
/// Uses position matching (same logic as `apply_pipe_radius_model`) to detect parent-child.
/// Returns root-level nodes (stems).
pub fn segments_to_tree(segments: &[BranchSegment]) -> Vec<BranchNode> {
    if segments.is_empty() {
        return vec![];
    }

    let epsilon = 0.01f32;

    // Build parent lookup: for each segment index, which segment is its parent?
    let mut parent_of: Vec<Option<usize>> = vec![None; segments.len()];
    let mut children_of: Vec<Vec<usize>> = vec![Vec::new(); segments.len()];

    // M6 fix: Match children to parents using closest-point-on-segment, not just endpoint
    // This allows children to attach anywhere along the parent segment
    let mut child_position_in_parent: Vec<f32> = vec![1.0; segments.len()];

    for (child_idx, child) in segments.iter().enumerate() {
        let mut best_dist = f32::MAX;
        let mut best_parent = None;
        let mut best_pos = 1.0f32;

        for (parent_idx, parent) in segments.iter().enumerate() {
            if parent_idx == child_idx {
                continue;
            }
            // Project child.start onto parent segment to find closest point
            let to_child = child.start - parent.start;
            let along = to_child.dot(parent.direction) / parent.length;
            let along_clamped = along.clamp(0.0, 1.0);
            let closest_point = parent.start + parent.direction * parent.length * along_clamped;
            let distance = (child.start - closest_point).length();

            if distance < epsilon && distance < best_dist {
                best_dist = distance;
                best_parent = Some(parent_idx);
                best_pos = along_clamped;
            }
        }

        if let Some(parent_idx) = best_parent {
            parent_of[child_idx] = Some(parent_idx);
            children_of[parent_idx].push(child_idx);
            child_position_in_parent[child_idx] = best_pos;
        }
    }

    // Build nodes bottom-up (leaves first) using topological sort.
    // This ensures children are always constructed before their parents,
    // regardless of depth values (critical for multi-segment continuation chains).
    let mut sorted: Vec<usize> = Vec::with_capacity(segments.len());
    let mut processed = vec![false; segments.len()];
    let mut queue: std::collections::VecDeque<usize> = std::collections::VecDeque::new();

    // Start with leaves (segments that have no children)
    for i in 0..segments.len() {
        if children_of[i].is_empty() {
            queue.push_back(i);
        }
    }
    while let Some(idx) = queue.pop_front() {
        sorted.push(idx);
        processed[idx] = true;
        if let Some(parent) = parent_of[idx] {
            // Add parent to queue once all its children are processed
            if !processed[parent] && children_of[parent].iter().all(|&c| processed[c]) {
                queue.push_back(parent);
            }
        }
    }
    // Add any remaining unprocessed nodes (disconnected roots)
    for i in 0..segments.len() {
        if !processed[i] {
            sorted.push(i);
        }
    }

    let mut nodes: Vec<Option<BranchNode>> = vec![None; segments.len()];

    for &idx in &sorted {
        let seg = &segments[idx];
        let tangent = get_perpendicular(seg.direction);

        let mut children: Vec<BranchNodeChild> = Vec::new();

        // Collect children, putting the continuation child first (closest to parent end direction)
        let child_indices = &children_of[idx];
        if !child_indices.is_empty() {
            // Find continuation child: the one whose direction is most aligned with parent
            let mut best_continuation = 0usize;
            let mut best_dot = -2.0f32;
            for (i, &ci) in child_indices.iter().enumerate() {
                let dot = seg.direction.dot(segments[ci].direction);
                if dot > best_dot {
                    best_dot = dot;
                    best_continuation = i;
                }
            }

            // Add continuation first
            let cont_idx = child_indices[best_continuation];
            if let Some(cont_node) = nodes[cont_idx].take() {
                children.push(BranchNodeChild {
                    node: cont_node,
                    position_in_parent: child_position_in_parent[cont_idx],
                });
            }

            // Add side branches
            for (i, &ci) in child_indices.iter().enumerate() {
                if i == best_continuation {
                    continue;
                }
                if let Some(side_node) = nodes[ci].take() {
                    // M6 fix: Use computed position_in_parent from segment projection
                    children.push(BranchNodeChild {
                        node: side_node,
                        position_in_parent: child_position_in_parent[ci],
                    });
                }
            }
        }

        nodes[idx] = Some(BranchNode {
            direction: seg.direction,
            tangent,
            length: seg.length,
            radius: seg.base_radius,
            depth: seg.depth,
            height_ratio: seg.height_ratio,
            children,
        });
    }

    // Collect root nodes (those with no parent)
    let mut roots = Vec::new();
    for (idx, parent) in parent_of.iter().enumerate() {
        if parent.is_none() {
            if let Some(node) = nodes[idx].take() {
                roots.push(node);
            }
        }
    }

    roots
}

/// Convert a GrowthNode tree directly into a BranchNode tree (already hierarchical).
#[allow(dead_code)]
pub fn growth_node_to_branch_node(node: &crate::growth::GrowthNode) -> Option<BranchNode> {
    if !node.is_renderable() || node.length < 0.01 {
        // Still process children to find renderable subtrees
        let child_nodes: Vec<BranchNode> = node
            .children
            .iter()
            .filter_map(growth_node_to_branch_node)
            .collect();
        if child_nodes.is_empty() {
            return None;
        }
        // If this node isn't renderable but has renderable children, return first child
        // with remaining as its side branches (best effort)
        if child_nodes.len() == 1 {
            return Some(child_nodes.into_iter().next().unwrap());
        }
        let mut first = child_nodes.into_iter();
        let mut root = first.next().unwrap();
        for sibling in first {
            root.children.push(BranchNodeChild {
                node: sibling,
                position_in_parent: 0.0,
            });
        }
        return Some(root);
    }

    let tangent = get_perpendicular(node.direction);

    // Convert children
    let mut children: Vec<BranchNodeChild> = Vec::new();
    for child in &node.children {
        if let Some(child_node) = growth_node_to_branch_node(child) {
            children.push(BranchNodeChild {
                node: child_node,
                position_in_parent: child.position_in_parent,
            });
        }
    }

    // Sort: continuation (highest direction alignment) first
    if children.len() > 1 {
        let parent_dir = node.direction;
        children.sort_by(|a, b| {
            let dot_a = parent_dir.dot(a.node.direction);
            let dot_b = parent_dir.dot(b.node.direction);
            dot_b
                .partial_cmp(&dot_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    Some(BranchNode {
        direction: node.direction,
        tangent,
        length: node.length,
        radius: node.radius,
        depth: 0, // Will be set by caller if needed
        height_ratio: (node.position.y / 5.0).clamp(0.0, 1.0), // Approximate
        children,
    })
}

// ════════════════════════════════════════════════════════════
// Core Mesher Functions (ported from C++ ManifoldMesher)
// ════════════════════════════════════════════════════════════

fn get_smooth_amount(radius: f32, node_length: f32) -> f32 {
    (radius / node_length.max(0.001)).min(1.0)
}

/// Calculate total extent (length) of a branch following continuation children
fn calculate_branch_extent(node: &BranchNode) -> f32 {
    let mut extent = node.length;
    if !node.is_leaf() && !node.children.is_empty() {
        extent += calculate_branch_extent(&node.children[0].node);
    }
    extent
}

/// Get position at a factor along a node
fn get_position_in_node(node_position: Vector3, node: &BranchNode, factor: f32) -> Vector3 {
    node_position + node.direction * node.length * factor
}

/// Create a ring of vertices around a cross-section of the branch.
fn add_circle(
    node_position: Vector3,
    node: &BranchNode,
    factor: f32,
    radial_n: usize,
    mesh: &mut ManifoldMeshData,
    uv_y: f32,
    pp_ctx: &PivotPainterContext,
    _pivot_painter_enabled: bool,
) -> CircleDesignator {
    let right = node.tangent;
    let vertex_index = mesh.mesh.vertices.len();
    let uv_index = mesh.uv_list.len();
    let up = node.tangent.cross(node.direction);
    let circle_position = node_position + node.length * factor * node.direction;

    // Interpolate radius along node
    let radius = if node.is_leaf() {
        node.radius
    } else {
        let child_radius = node.children[0].node.radius;
        node.radius + (child_radius - node.radius) * factor
    };

    let smooth_amount = get_smooth_amount(radius, node.length);

    // Generate radial_n + 1 vertices: duplicate vertex 0 at the end for UV seam
    for i in 0..=radial_n {
        let angle = (i % radial_n) as f32 / radial_n as f32 * TAU;
        let point = right * angle.cos() + up * angle.sin();
        let vertex = point * radius + circle_position;
        let idx = mesh.add_vertex(vertex);
        mesh.mesh.smooth_weights[idx] = smooth_amount;
        mesh.radii[idx] = radius; // L4: Store per-vertex radius

        // C++ always assigns pivot painter attributes unconditionally
        mesh.custom0[idx] = [
            pp_ctx.stem_id as f32,
            pp_ctx.hierarchy_depth as f32,
            pp_ctx.branch_extent,
            0.0,
        ];
        mesh.custom1[idx] = [
            pp_ctx.pivot_position.x,
            pp_ctx.pivot_position.y,
            pp_ctx.pivot_position.z,
            0.0,
        ];
        mesh.directions[idx] = node.direction;

        // UV: seam vertex (i == radial_n) gets u=1.0
        let uv_x = i as f32 / radial_n as f32;
        mesh.uv_list.push(Vector2::new(uv_x, uv_y));
    }

    CircleDesignator {
        vertex_index,
        uv_index,
        radial_n,
    }
}

fn is_index_in_branch_mask(mask: &[IndexRange], index: i32, radial_n: i32) -> bool {
    let offset = radial_n / 2;
    for range in mask {
        let mut i = index;
        let mut min_idx = range.min_index;
        let mut max_idx = range.max_index;
        if max_idx < min_idx {
            i = (i + offset) % radial_n;
            min_idx = (min_idx + offset) % radial_n;
            max_idx = (max_idx + offset) % radial_n;
        }
        if i >= min_idx && i < max_idx {
            return true;
        }
    }
    false
}

/// Connect two vertex rings with quads, optionally skipping masked regions where branches attach.
fn bridge_circles(
    first: &CircleDesignator,
    second: &CircleDesignator,
    radial_n: usize,
    mesh: &mut ManifoldMeshData,
    mask: Option<&[IndexRange]>,
) {
    for i in 0..radial_n as i32 {
        if let Some(m) = mask {
            if is_index_in_branch_mask(m, i, radial_n as i32) {
                continue;
            }
        }
        // With radial_n+1 vertices per circle, seam vertex exists at index radial_n
        let v0 = first.vertex_index + i as usize;
        let v1 = first.vertex_index + (i + 1) as usize;
        let v2 = second.vertex_index + (i + 1) as usize;
        let v3 = second.vertex_index + i as usize;
        mesh.add_quad(v0, v1, v2, v3);
    }
}

/// Project branch direction onto plane perpendicular to parent, return angle via atan2.
fn get_branch_angle_around_parent(parent: &BranchNode, branch: &BranchNode) -> f32 {
    // Project branch direction onto plane perpendicular to parent direction
    let dot = branch.direction.dot(parent.direction);
    let projected = (branch.direction - parent.direction * dot).normalized();

    let right = parent.tangent;
    let up = right.cross(parent.direction);
    let cos_angle = projected.dot(right);
    let sin_angle = projected.dot(up);
    (sin_angle.atan2(cos_angle) + TAU) % TAU
}

/// Map child branch angular footprint to radial index range on parent circle.
fn get_branch_indices_on_circle(
    radial_n: usize,
    circle_radius: f32,
    branch_radius: f32,
    branch_angle: f32,
) -> IndexRange {
    let angle_delta = (branch_radius / circle_radius).clamp(-1.0, 1.0).asin();
    let increment = TAU / radial_n as f32;
    let min_index = ((branch_angle - angle_delta + TAU) % TAU / increment) as i32;
    let max_index = ((branch_angle + angle_delta + increment + TAU) % TAU / increment) as i32;
    IndexRange {
        min_index,
        max_index,
    }
}

/// For each side branch child, compute its IndexRange on the parent circle.
fn get_children_ranges(node: &BranchNode, radial_n: usize) -> Vec<IndexRange> {
    let mut ranges = Vec::new();
    for i in 1..node.children.len() {
        let child = &node.children[i].node;
        let angle = get_branch_angle_around_parent(node, child);
        let range = get_branch_indices_on_circle(radial_n, node.radius, child.radius, angle);
        ranges.push(range);
    }
    ranges
}

/// Map parent circle vertices to child circle vertices at junction.
fn get_child_index_order(
    parent_base: &CircleDesignator,
    child_radial_n: usize,
    child_range: &IndexRange,
) -> Vec<usize> {
    let mut child_base_indices = vec![0usize; child_radial_n];
    // Stride is radial_n + 1 due to duplicated seam vertex
    let stride = parent_base.radial_n + 1;

    for i in 0..(child_radial_n / 2) {
        let lower_index = ((child_range.min_index as usize + i) % parent_base.radial_n)
            + parent_base.vertex_index;
        let upper_index = lower_index + stride;

        child_base_indices[i] = lower_index;
        child_base_indices[child_radial_n - i - 1] = upper_index;
    }
    child_base_indices
}

/// Create junction quads connecting parent circle to child circle.
fn add_child_base_geometry(
    child_base_indices: &[usize],
    child_base: &CircleDesignator,
    child_radius: f32,
    child_pos: Vector3,
    offset: usize,
    smooth_amount: f32,
    mesh: &mut ManifoldMeshData,
    child_uvs: &[Vector2],
    pp_ctx: &PivotPainterContext,
    _pivot_painter_enabled: bool,
    _direction: Vector3,
) {
    // Calculate center of the child base polygon
    let mut child_base_center = Vector3::ZERO;
    for &i in child_base_indices {
        child_base_center += mesh.mesh.vertices[i];
    }
    child_base_center /= child_base_indices.len() as f32;

    let radial_n = child_base.radial_n;

    // H4: Compute junction direction from polygon cross product (actual geometry)
    // instead of using child branch direction
    let junction_direction = if child_base_indices.len() >= 3 {
        let v0 = mesh.mesh.vertices[child_base_indices[0]];
        let v1 = mesh.mesh.vertices[child_base_indices[1]];
        let v2 = mesh.mesh.vertices[child_base_indices[2]];
        let edge1 = v2 - v0;
        let edge2 = v1 - v0;
        let normal = edge1.cross(edge2);
        if normal.length_squared() > 0.0001 {
            normal.normalized()
        } else {
            _direction // Fallback to passed direction if geometry is degenerate
        }
    } else {
        _direction
    };

    for i in 0..radial_n {
        let index = (i + offset) % radial_n;
        let vertex = mesh.mesh.vertices[child_base_indices[index]];
        let vertex = (vertex - child_base_center).normalized() * child_radius + child_pos;
        let added_idx = mesh.add_vertex(vertex);
        mesh.mesh.smooth_weights[added_idx] = smooth_amount;
        mesh.radii[added_idx] = child_radius; // L4: Store per-vertex radius at junction

        // M10 fix: Push UV inline with vertex to maintain 1:1 mapping
        if i < child_uvs.len() {
            mesh.uv_list.push(child_uvs[i]);
        } else {
            mesh.uv_list
                .push(Vector2::new(i as f32 / radial_n as f32, 0.0));
        }

        // C++ always assigns pivot painter attributes unconditionally
        mesh.custom0[added_idx] = [
            pp_ctx.stem_id as f32,
            pp_ctx.hierarchy_depth as f32,
            pp_ctx.branch_extent,
            0.0,
        ];
        mesh.custom1[added_idx] = [
            pp_ctx.pivot_position.x,
            pp_ctx.pivot_position.y,
            pp_ctx.pivot_position.z,
            0.0,
        ];
        // H4: Use geometry-derived junction direction
        mesh.directions[added_idx] = junction_direction;

        // Quad connecting parent ring to child ring
        let v0 = child_base_indices[index];
        let v1 = child_base_indices[(index + 1) % radial_n];
        let v2 = child_base.vertex_index + (i + 1) % radial_n;
        let v3 = child_base.vertex_index + i;
        mesh.add_quad(v0, v1, v2, v3);
    }
}

/// Get twist angle of child relative to parent
fn get_child_twist(child: &BranchNode, parent: &BranchNode) -> f32 {
    // Project parent direction onto plane perpendicular to child direction
    let dot = parent.direction.dot(child.direction);
    let projected = (parent.direction - child.direction * dot).normalized();

    let right = projected;
    let up = right.cross(child.direction);
    let cos_angle = child.tangent.dot(right);
    let sin_angle = child.tangent.dot(up);
    (sin_angle.atan2(cos_angle) + TAU) % TAU
}

/// H5: Compute UVs for child base vertices at junction.
///
/// Godot requires 1:1 vertex-to-UV mapping, so we can't use C++'s per-polygon UV loops.
/// This improved version computes UVs based on the junction's position in UV space,
/// accounting for the parent's UV growth rate.
///
/// Parameters:
/// - parent_uv_y: Current V coordinate on the parent branch
/// - child_radial_n: Number of radial segments on child
/// - parent_radius: Parent branch radius at junction (for UV scale)
/// - child_radius: Child branch radius (for UV offset)
fn compute_child_base_uvs(
    parent_uv_y: f32,
    child_radial_n: usize,
    parent_radius: f32,
    child_radius: f32,
) -> Vec<Vector2> {
    let mut uvs = Vec::with_capacity(child_radial_n);

    // H5: Compute UV growth factor based on circumference ratio
    // This creates a smoother UV transition at the junction
    let uv_growth = child_radius / (parent_radius + 0.001) / std::f32::consts::TAU;
    let base_v = parent_uv_y + uv_growth * 0.5; // Offset by half the UV growth

    for i in 0..child_radial_n {
        let u = i as f32 / child_radial_n as f32;
        uvs.push(Vector2::new(u, base_v));
    }
    uvs
}

/// Orchestrate junction creation for a side branch child.
fn add_child_circle(
    parent: &BranchNode,
    child: &BranchNodeChild,
    child_pos: Vector3,
    _parent_pos: Vector3,
    parent_base: &CircleDesignator,
    child_range: &IndexRange,
    uv_y: f32,
    mesh: &mut ManifoldMeshData,
    pp_ctx: &PivotPainterContext,
    pivot_painter_enabled: bool,
) -> CircleDesignator {
    let smooth_amount = get_smooth_amount(child.node.radius, parent.length);

    // Number of vertices in child circle
    let child_radial_n = 2
        * (((child_range.max_index - child_range.min_index + parent_base.radial_n as i32)
            % parent_base.radial_n as i32
            + 1) as usize);
    let child_radial_n = child_radial_n.max(4); // Minimum 4 vertices

    let child_base_indices = get_child_index_order(parent_base, child_radial_n, child_range);

    let child_twist = get_child_twist(&child.node, parent);
    let offset = ((child_twist / TAU * child_radial_n as f32 - child_radial_n as f32 / 4.0
        + child_radial_n as f32) as usize)
        % child_radial_n;

    // H5: Compute child base UVs with improved UV scaling
    let child_uvs = compute_child_base_uvs(uv_y, child_radial_n, parent.radius, child.node.radius);

    let child_base = CircleDesignator {
        vertex_index: mesh.mesh.vertices.len(),
        uv_index: mesh.uv_list.len(), // Will be in sync after pushing below
        radial_n: child_radial_n,
    };

    add_child_base_geometry(
        &child_base_indices,
        &child_base,
        child.node.radius,
        child_pos,
        offset,
        smooth_amount,
        mesh,
        &child_uvs,
        pp_ctx,
        pivot_painter_enabled,
        child.node.direction,
    );

    child_base
}

/// Get position where a side branch attaches to parent surface
fn get_side_child_position(
    parent: &BranchNode,
    child: &BranchNodeChild,
    node_position: Vector3,
) -> Vector3 {
    // Project child direction onto plane perpendicular to parent
    let dot = child.node.direction.dot(parent.direction);
    let tangent = (child.node.direction - parent.direction * dot).normalized();
    node_position
        + parent.direction * parent.length * child.position_in_parent
        + tangent * parent.radius
}

/// Recursive traversal to build the manifold mesh.
fn mesh_node_rec(
    node: &BranchNode,
    node_position: Vector3,
    base: &CircleDesignator,
    mesh: &mut ManifoldMeshData,
    uv_y: f32,
    pp_ctx: &PivotPainterContext,
    stem_id_counter: &mut i32,
    radial_resolution: usize,
    pivot_painter_enabled: bool,
) {
    let uv_growth = node.length / (node.radius + 0.001) / TAU;

    if node.children.len() < 2 {
        // 0-1 children: simple bridge from base to end circle, recurse continuation
        let child_circle = add_circle(
            node_position,
            node,
            1.0,
            base.radial_n,
            mesh,
            uv_y + uv_growth,
            pp_ctx,
            pivot_painter_enabled,
        );
        bridge_circles(base, &child_circle, base.radial_n, mesh, None);

        let child_pos = get_position_in_node(node_position, node, 1.0);

        if !node.is_leaf() {
            mesh_node_rec(
                &node.children[0].node,
                child_pos,
                &child_circle,
                mesh,
                uv_y + uv_growth,
                pp_ctx,
                stem_id_counter,
                radial_resolution,
                pivot_painter_enabled,
            );
        }
    } else {
        // 2+ children: bridge with masks, create junction geometry for side branches
        let end_circle = add_circle(
            node_position,
            node,
            1.0,
            base.radial_n,
            mesh,
            uv_y + uv_growth,
            pp_ctx,
            pivot_painter_enabled,
        );
        let children_ranges = get_children_ranges(node, base.radial_n);
        bridge_circles(
            base,
            &end_circle,
            base.radial_n,
            mesh,
            Some(&children_ranges),
        );

        for i in 0..node.children.len() {
            if i == 0 {
                // Continuation child
                let child_pos = get_position_in_node(node_position, node, 1.0);
                mesh_node_rec(
                    &node.children[0].node,
                    child_pos,
                    &end_circle,
                    mesh,
                    uv_y + uv_growth,
                    pp_ctx,
                    stem_id_counter,
                    radial_resolution,
                    pivot_painter_enabled,
                );
            } else {
                // Side branch
                let child = &node.children[i];
                let child_pos = get_side_child_position(node, child, node_position);

                // Create new Pivot Painter context for side branch
                let child_pp_ctx = PivotPainterContext {
                    stem_id: {
                        *stem_id_counter += 1;
                        *stem_id_counter
                    },
                    hierarchy_depth: pp_ctx.hierarchy_depth + 1,
                    pivot_position: child_pos,
                    branch_extent: calculate_branch_extent(&child.node),
                };

                let child_base = add_child_circle(
                    node,
                    child,
                    child_pos,
                    node_position,
                    base,
                    &children_ranges[i - 1],
                    uv_y,
                    mesh,
                    &child_pp_ctx,
                    pivot_painter_enabled,
                );
                mesh_node_rec(
                    &child.node,
                    child_pos,
                    &child_base,
                    mesh,
                    uv_y + uv_growth,
                    &child_pp_ctx,
                    stem_id_counter,
                    radial_resolution,
                    pivot_painter_enabled,
                );
            }
        }
    }
}

// ════════════════════════════════════════════════════════════
// Public API
// ════════════════════════════════════════════════════════════

/// Configuration for manifold meshing
pub struct ManifoldMesherConfig {
    pub radial_resolution: usize,
    pub smooth_iterations: u32,
    pub smooth_factor: f32,
    pub pivot_painter_enabled: bool,
}

impl Default for ManifoldMesherConfig {
    fn default() -> Self {
        Self {
            radial_resolution: 8,
            smooth_iterations: 4,
            smooth_factor: 1.0,
            pivot_painter_enabled: false,
        }
    }
}

/// Generate a manifold mesh from a list of branch tree roots (stems).
///
/// Returns ManifoldMeshData with the mesh and optional pivot painter attributes.
pub fn mesh_tree(
    stems: &[BranchNode],
    stem_positions: &[Vector3],
    config: &ManifoldMesherConfig,
) -> ManifoldMeshData {
    let mut mesh = ManifoldMeshData::new();
    let mut stem_id_counter = 0i32;

    for (stem, &position) in stems.iter().zip(stem_positions.iter()) {
        if stem.children.is_empty() {
            continue;
        }

        let pp_ctx = PivotPainterContext {
            stem_id: {
                let id = stem_id_counter;
                stem_id_counter += 1;
                id
            },
            hierarchy_depth: 0,
            pivot_position: position,
            branch_extent: calculate_branch_extent(stem),
        };

        let start_circle = add_circle(
            position,
            stem,
            0.0,
            config.radial_resolution,
            &mut mesh,
            0.0,
            &pp_ctx,
            config.pivot_painter_enabled,
        );

        mesh_node_rec(
            stem,
            position,
            &start_circle,
            &mut mesh,
            0.0,
            &pp_ctx,
            &mut stem_id_counter,
            config.radial_resolution,
            config.pivot_painter_enabled,
        );
    }

    // Apply smoothing via the existing smoothing module
    if config.smooth_iterations > 0 && !mesh.mesh.vertices.is_empty() {
        crate::smoothing::laplacian_smooth_weighted(
            &mut mesh.mesh.vertices,
            &mesh.mesh.indices,
            &mesh.mesh.smooth_weights,
            config.smooth_iterations,
            config.smooth_factor,
        );
    }

    // Recalculate normals after smoothing
    if !mesh.mesh.vertices.is_empty() {
        crate::smoothing::recalculate_normals(
            &mesh.mesh.vertices,
            &mesh.mesh.indices,
            &mut mesh.mesh.normals,
        );
    }

    // M10 fix: UVs are now pushed inline with vertices (1:1 mapping maintained).
    // Copy uv_list directly; pad or truncate to match vertex count as safety net.
    mesh.mesh.uvs = mesh.uv_list.clone();
    mesh.mesh
        .uvs
        .resize(mesh.mesh.vertices.len(), Vector2::ZERO);

    mesh
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_segments_to_tree_single() {
        let segments = vec![BranchSegment {
            start: Vector3::ZERO,
            direction: Vector3::UP,
            length: 1.0,
            base_radius: 0.1,
            tip_radius: 0.05,
            depth: 0,
            is_terminal: true,
            height_ratio: 0.5,
            subtree_weight: 1.0,
        }];
        let tree = segments_to_tree(&segments);
        assert_eq!(tree.len(), 1);
        assert!(tree[0].is_leaf());
    }

    #[test]
    fn test_segments_to_tree_parent_child() {
        let segments = vec![
            BranchSegment {
                start: Vector3::ZERO,
                direction: Vector3::UP,
                length: 1.0,
                base_radius: 0.2,
                tip_radius: 0.1,
                depth: 0,
                is_terminal: false,
                height_ratio: 0.5,
                subtree_weight: 1.5,
            },
            BranchSegment {
                start: Vector3::new(0.0, 1.0, 0.0),
                direction: Vector3::new(1.0, 1.0, 0.0).normalized(),
                length: 0.5,
                base_radius: 0.1,
                tip_radius: 0.05,
                depth: 1,
                is_terminal: true,
                height_ratio: 0.8,
                subtree_weight: 0.5,
            },
        ];
        let tree = segments_to_tree(&segments);
        assert_eq!(tree.len(), 1); // One root
        assert_eq!(tree[0].children.len(), 1); // With one child
    }

    #[test]
    fn test_mesh_tree_produces_geometry() {
        let node = BranchNode {
            direction: Vector3::UP,
            tangent: Vector3::RIGHT,
            length: 1.0,
            radius: 0.1,
            depth: 0,
            height_ratio: 0.5,
            children: vec![BranchNodeChild {
                node: BranchNode {
                    direction: Vector3::UP,
                    tangent: Vector3::RIGHT,
                    length: 0.5,
                    radius: 0.05,
                    depth: 1,
                    height_ratio: 0.8,
                    children: vec![],
                },
                position_in_parent: 1.0,
            }],
        };

        let config = ManifoldMesherConfig {
            radial_resolution: 6,
            smooth_iterations: 0,
            ..Default::default()
        };

        let result = mesh_tree(&[node], &[Vector3::ZERO], &config);
        assert!(!result.mesh.vertices.is_empty());
        assert!(!result.mesh.indices.is_empty());
    }

    #[test]
    fn test_mesh_tree_with_side_branch() {
        let node = BranchNode {
            direction: Vector3::UP,
            tangent: Vector3::RIGHT,
            length: 1.0,
            radius: 0.2,
            depth: 0,
            height_ratio: 0.5,
            children: vec![
                // Continuation
                BranchNodeChild {
                    node: BranchNode {
                        direction: Vector3::UP,
                        tangent: Vector3::RIGHT,
                        length: 0.5,
                        radius: 0.15,
                        depth: 0,
                        height_ratio: 0.7,
                        children: vec![],
                    },
                    position_in_parent: 1.0,
                },
                // Side branch
                BranchNodeChild {
                    node: BranchNode {
                        direction: Vector3::new(1.0, 0.5, 0.0).normalized(),
                        tangent: Vector3::new(0.0, 0.0, 1.0),
                        length: 0.4,
                        radius: 0.08,
                        depth: 1,
                        height_ratio: 0.6,
                        children: vec![],
                    },
                    position_in_parent: 0.7,
                },
            ],
        };

        let config = ManifoldMesherConfig {
            radial_resolution: 8,
            smooth_iterations: 0,
            ..Default::default()
        };

        let result = mesh_tree(&[node], &[Vector3::ZERO], &config);
        assert!(!result.mesh.vertices.is_empty());
        assert!(!result.mesh.indices.is_empty());
        // Side branch junction should create more geometry than simple bridge
        assert!(result.mesh.vertices.len() > 20);
    }

    #[test]
    fn test_branch_angle_around_parent() {
        let parent = BranchNode {
            direction: Vector3::UP,
            tangent: Vector3::RIGHT,
            length: 1.0,
            radius: 0.2,
            depth: 0,
            height_ratio: 0.5,
            children: vec![],
        };
        let branch = BranchNode {
            direction: Vector3::RIGHT,
            tangent: Vector3::UP,
            length: 0.5,
            radius: 0.1,
            depth: 1,
            height_ratio: 0.5,
            children: vec![],
        };
        let angle = get_branch_angle_around_parent(&parent, &branch);
        assert!(angle >= 0.0 && angle < TAU);
    }
}
