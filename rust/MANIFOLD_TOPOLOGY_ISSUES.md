# Manifold Mesh Topology Implementation

## Overview

The manifold mesh topology feature (`manifold_topology = true`) creates watertight meshes where branches properly connect to the trunk with shared vertices at hole boundaries. This document describes the implementation and remaining limitations.

## What Manifold Topology Achieves

The goal is to create a "watertight" mesh where:
1. Branches share vertices with the trunk at attachment points
2. Holes are cut in the trunk where branches attach
3. Proper transition geometry bridges trunk-space to branch-space
4. Junction vertices can be smoothed for better visual quality

This enables proper physics simulation, correct ambient occlusion, and cleaner mesh topology.

## Implementation Details

### Multi-Ring Hole Detection (Fixed)

**Problem:** Holes were only marked on ONE trunk ring, but quads span TWO adjacent rings.

**Solution:** Now marks holes on BOTH `ring_idx` AND `ring_idx + 1`:
```rust
let ring_indices = [ring_idx, (ring_idx + 1).min(rings - 1)];
for &ring in &ring_indices {
    // Mark segments as holes
}
```

**Location:** `tree.rs:create_trunk_mesh_with_holes()`

### HoleBoundary Struct (New)

**Problem:** The old `VertexRing` only contained vertices from a single ring.

**Solution:** New `HoleBoundary` struct contains:
- `lower_indices`: Vertices from the lower trunk ring
- `upper_indices`: Vertices from the upper trunk ring
- Metadata: center, direction, branch_radius, trunk_radius, arc_extent, arc_start_angle

The `get_loop_indices()` method returns a closed loop: lower ring (segment order) + upper ring (reversed).

**Location:** `branch.rs:HoleBoundary`

### Transition Geometry (New)

**Problem:** There was no smooth transition between trunk-space and branch-space coordinate systems.

**Solution:** `generate_branch_mesh_manifold()` now creates:
1. **Transition ring**: Circular ring in branch-space slightly offset from the branch start
2. **Hole-to-transition bridge**: Triangles connecting the irregular hole boundary loop to the circular transition ring
3. **Branch cylinder**: Standard branch rings from the transition ring to the tip

This creates a saddle-shaped transition surface that properly bridges the two coordinate systems.

**Location:** `branch.rs:generate_branch_mesh_manifold()`

### Junction Smoothing (New)

**Problem:** Sharp transitions at junction vertices.

**Solution:** Laplacian smoothing applied to junction vertices:
- Controlled by `junction_blend` export (0.0-1.0)
- Moves vertices toward the average position of neighbors
- Recalculates normals after smoothing

**Location:** `smoothing.rs`, integrated in `tree.rs:generate_manifold()`

## Current Status

| File | Function | Status |
|------|----------|--------|
| `tree.rs` | `generate_manifold()` | **Working** - with new transition geometry |
| `tree.rs` | `create_trunk_mesh_with_holes()` | **Working** - multi-ring hole detection |
| `branch.rs` | `generate_branch_mesh_manifold()` | **Working** - proper transition geometry |
| `branch.rs` | `HoleBoundary` | **New** - replaces VertexRing for manifold |
| `branch.rs` | `calculate_hole_segments()` | **Working** - unchanged |
| `smoothing.rs` | `laplacian_smooth()` | **New** - junction smoothing |
| `tree.rs` | `generate()` | **Working** - non-manifold approach |
| `junction.rs` | `generate_branch_collar()` | **Working** - alternative solution |

## Remaining Limitations

1. **Elliptical Intersection**: The hole is still approximated by trunk segments rather than a true elliptical cylinder intersection.

2. **Segment Alignment**: The number of hole boundary vertices may not perfectly match the branch ring segment count, requiring triangle bridging.

3. **Complex Angles**: Very steep branch angles (close to 90°) may produce suboptimal transition geometry.

## Usage

To enable manifold topology:
```gdscript
pixy_tree.manifold_topology = true
pixy_tree.junction_blend = 0.3  # Optional: smooth junctions
pixy_tree.generate()
```

For maximum visual quality without manifold requirements, use:
```gdscript
pixy_tree.manifold_topology = false  # (default)
pixy_tree.branch_collar_enabled = true  # (default)
```

## Testing

Run the test suite to verify manifold functionality:
```bash
cd rust && cargo test
```

Key tests:
- `test_hole_boundary_creation`
- `test_hole_boundary_loop_indices`
- `test_calculate_hole_segments_wraparound`
- `test_laplacian_smooth_preserves_non_junction`
