# C++ modular_tree vs Rust Parity Review

**Generated:** 2026-02-05
**Reviewers:** 8 parallel agents analyzing GrowthFunction, BranchFunction, ManifoldMesher, Property, CrownShape, Tree Defaults, Helper Functions, Smoothing/Foliage

---

## Executive Summary

| Severity | Count | Description |
|----------|-------|-------------|
| **Critical** | 0 | None |
| **High** | 2 | Virtual nodes, cut_threshold default |
| **Medium** | 18 | Various algorithmic and default differences |
| **Low** | 25+ | Minor differences, documented gaps, enhancements |

**Overall Status:** Core algorithms have good parity. Main gaps are:
1. Virtual node placement (multiple origins per segment)
2. Default value mismatches (especially gravity, resolution, flatness)
3. UV coordinate calculation at branch junctions (architectural)

---

## Priority 1: High Severity Gaps

### [B08] Virtual Nodes Not Implemented
**Severity:** High
**C++ Location:** BranchFunction.cpp:288-376
**Rust Location:** branch.rs:667-929
**Issue:** C++ places multiple branch origins per parent segment ("virtual nodes"). Rust places exactly one origin per loop iteration. This affects branch density distribution along the trunk.

**C++ Code:**
```cpp
int origins_to_create = remaining_node_length / origins_dist + 1;
float position_in_parent_step = origins_dist / node.length;

for (int i = 0; i < origins_to_create; i++)
{
    tangent = rot * tangent;
    Geometry::project_on_plane(tangent, node.direction);
    // Create multiple origins along single segment
    position_in_parent += position_in_parent_step;
}
```

**Fix:** Implement multi-origin placement per segment:
1. Track `current_length` and `dist_to_next_origin` along parent
2. Create multiple origins when a parent segment spans multiple origin positions
3. Use `position_in_parent` for precise placement within segments

---

### [T13] Cut Threshold Default Mismatch
**Severity:** High
**C++ Location:** GrowthFunction.hpp:51
**Rust Location:** tree.rs:764
**Issue:** Cut threshold default differs significantly, affecting pruning behavior.

| Implementation | Default |
|----------------|---------|
| C++ | 0.2 |
| Rust | 0.1 |

**Fix:** Update Rust default to `0.2` for C++ parity.

---

## Priority 2: Medium Severity Gaps

### Growth Function

#### [G40] get_look_at_rot Implementation Difference
**C++ Location:** GeometryUtilities.cpp:30-44
**Rust Location:** growth.rs:693-717
**Issue:** C++ uses axis-angle rotation from `UP.cross(direction)`. Rust builds orthonormal basis. Mathematically similar but edge case handling differs.

**Fix:** Align edge case thresholds and verify results match for near-vertical directions.

#### [G51] Missing suppress_tip_growth Logic
**C++ Location:** GrowthFunction.cpp:13-24, 320
**Rust Location:** MISSING
**Issue:** C++ marks tips as `Ignored` when lateral branching is enabled (`suppress_tip_growth`). Rust always has apical meristem as `Meristem`.

**Fix:** When lateral branching is enabled, mark trunk tip as `Ignored` instead of `Meristem`.

---

### Branch Function

#### [B07] Node Tangent Not Tracked
**C++ Location:** BranchFunction.cpp:170, 192-193
**Rust Location:** MISSING in BranchSegment
**Issue:** C++ propagates `node.tangent` from parent to child for UV and phyllotaxis consistency. Rust BranchSegment has no tangent field.

**Fix:** Add `tangent: Vector3` field to BranchSegment, propagate from parent to children.

#### [B09] Origins on Straight Trunk, Not Curved Parent
**C++ Location:** BranchFunction.cpp:260-268
**Rust Location:** branch.rs:730-734
**Issue:** C++ places origins along actual curved parent branch geometry. Rust places origins along straight vertical trunk.

**Fix:** Pass actual trunk node chain (with positions) to origin generation rather than computing trunk surface position analytically. (Documented gap C1 in MEMORY.md)

---

### Helper Functions

#### [H10] random_vec Normalization Inconsistency
**C++ Location:** GeometryUtilities.cpp:54-60
**Rust Location:** branch.rs:261-269
**Issue:** C++ `random_vec(flatness)` does NOT normalize. Rust always normalizes.

**C++ Code:**
```cpp
Vector3 random_vec(float flatness) {
    auto vec = Vector3{};
    vec.setRandom();  // [-1,1] per axis
    vec.z() *= (1 - flatness);
    return vec;  // NOT normalized
}
```

**Fix:** Remove normalization from Rust `random_vec`, let callers normalize when needed.

#### [H11] get_orthogonal_vector Cross Product Order
**C++ Location:** GeometryUtilities.cpp:70-82
**Rust Location:** branch.rs:272-280
**Issue:** C++ uses `tmp.cross(v)` while Rust uses `dir.cross(up)`. Cross product is anti-commutative, producing opposite-direction perpendicular vectors.

**Fix:** Change Rust to `up.cross(dir).normalized()`.

---

### Tree Defaults

| ID | Parameter | C++ Default | Rust Default | Fix |
|----|-----------|-------------|--------------|-----|
| T01 | trunk_height | 10.0 | 5.0 | Update to 10.0 |
| T02 | trunk_radius | 0.3 | 0.5 | Update to 0.3 |
| T08 | branch_radius_ratio | 0.4 | 0.3 | Update to 0.4 |
| T10 | branch_resolution | 3.0 | 0.0 | Update to 3.0 |
| T11 | branch_flatness | 0.5 | 0.0 | Update to 0.5 |
| T12 | gravity_strength | 10.0 | 0.0 | Update to 10.0 or document |

---

### ManifoldMesher

#### [M20] UV Calculation for Child Base Vertices
**C++ Location:** ManifoldMesher.cpp:271-308
**Rust Location:** manifold_mesher.rs:655-673
**Issue:** C++ generates UVs in three phases: outer (semi-circles), inner (collar), standard circle. Rust uses simplified linear distribution.

**Fix:** Due to Godot's 1:1 vertex-to-UV requirement, full C++ UV scheme cannot be directly ported. Document UV mapping difference at junctions.

#### [M21] UV Loops vs Per-Vertex UVs
**Issue:** C++ uses per-polygon UV indices (`mesh.uv_loops`). Godot requires 1:1 vertex-to-UV. Rust duplicates vertices at UV seams (`radial_n + 1` instead of `radial_n`).

**Status:** Architectural difference. Current workaround (vertex duplication) is appropriate for Godot.

---

### Property System

#### [P02] RandomProperty Uses Separate Unseeded RNG
**C++ Location:** Property.hpp:25-34
**Rust Location:** property.rs:39
**Issue:** C++ `RandomProperty` has its own unseeded RNG (non-reproducible). Rust shares the main seeded RNG (reproducible).

**Recommendation:** Keep Rust behavior (reproducible). Document as intentional improvement.

---

### Crown Shape

All formulas match exactly. Minor issues:

| ID | Issue | Severity |
|----|-------|----------|
| CS03 | Documentation says "0=bottom, 1=top" but code uses inverted convention | Low |
| CS04 | Missing named constants (MIN_RATIO, etc.) | Low |
| CS05 | No CrownParams struct (parameters scattered) | Low |

**Enhancements in Rust:** `Spreading` and `Umbrella` shapes, `crown_influence` blend factor.

---

## Priority 3: Low Severity Gaps

### Growth Function
- **G42:** Extension phyllotaxis angle on split - minor difference
- **G43:** Split tangent inheritance - different approach

### Branch Function
- **B06:** Split position_in_parent is random in C++, fixed in Rust
- **B11:** Missing propagate_inactive_rec before gravity
- **B15:** end_radius vs branch_taper semantic difference
- **B17:** get_perpendicular cross product order

### Tree Defaults
| Parameter | C++ | Rust |
|-----------|-----|------|
| break_chance | 0.01 | 0.0 |
| flower_threshold | 0.5 | 0.15 |
| growth_randomness | 0.1 | 0.2 |
| pipe_radius_min | 0.01 | 0.02 |

### Helper Functions
- **H12:** lerp_vec3 clamping (documented M3)
- **H13:** Smoothing adjacency edge skipping (documented M9)
- **H14:** get_look_at_rot threshold differences
- **H16:** project_on_plane not extracted as utility

### Smoothing
- **S01:** Polygon edge skipping (quad vs triangle topology)
- **S02:** Double-buffer vs new allocation (performance only)

---

## Not Gaps (Verified Matching)

These items were verified to match C++ behavior:

- Crown shape formulas (all 8 shapes)
- Vigor distribution algorithm
- Secondary growth formula
- Dynamic cut threshold logic
- Gravity tangent axis (correct Y-up conversion)
- Property evaluation factor calculation
- Smooth iterations default (4)
- Radial resolution default (8)
- lerp() clamping for float (M3 fix)
- Phyllotaxis persistent tangent (A5 fix)

---

## Rust Enhancements (Not in C++)

These features exist in Rust but not in C++:

1. **Foliage System** - Complete original implementation (C++ has empty files)
   - 6 leaf styles (CrossedPlanes, SingleQuad, ClusterSphere, etc.)
   - 3 placement modes (TerminalBranches, AllBranches, TipClusters)
   - 4 orientation modes
   - 30+ tree species presets

2. **Additional Crown Shapes** - Spreading, Umbrella

3. **Crown Influence Parameter** - Blend factor for crown shape effect

4. **Reproducible Randomness** - All RNG uses seeded source

5. **TaperMode Enum** - Legacy vs EndRadius branch taper semantics

---

## Recommended Fix Order

### Batch 1: Quick Fixes (Defaults)
```rust
// tree.rs - Update these defaults:
cut_threshold: 0.2,        // was 0.1
branch_flatness: 0.5,      // was 0.0
branch_resolution: 3.0,    // was 0.0
branch_radius_ratio: 0.4,  // was 0.3
```

### Batch 2: Helper Functions
1. Fix `get_perpendicular` cross product order: `up.cross(dir)`
2. Remove normalization from `random_vec`, normalize at call sites

### Batch 3: Medium Algorithmic Changes
1. Add `suppress_tip_growth` logic (G51)
2. Add `tangent` field to BranchSegment (B07)

### Batch 4: Larger Features (Optional)
1. Virtual nodes implementation (B08) - significant effort
2. Origins along curved parent (B09) - requires architecture change

---

## Files Changed Summary

| File | Gap Count | Priority Fixes |
|------|-----------|----------------|
| tree.rs | 12 | Default value updates |
| branch.rs | 8 | get_perpendicular, random_vec, tangent field |
| growth.rs | 5 | suppress_tip_growth |
| manifold_mesher.rs | 4 | UV documentation |
| property.rs | 2 | (keep current behavior) |
| crown_shape.rs | 3 | Documentation only |
| smoothing.rs | 2 | (architectural, keep) |

---

## Detailed Findings by Agent

### GrowthFunction Agent Findings (G40-G55)

| ID | Issue | Severity | Status |
|----|-------|----------|--------|
| G40 | get_look_at_rot implementation | Medium | Gap |
| G41 | random_vec not normalized | Medium | False positive |
| G42 | Extension phyllotaxis on split | Low | Minor diff |
| G43 | Split tangent inheritance | Low | Different approach |
| G51 | suppress_tip_growth missing | Medium | Gap |

### BranchFunction Agent Findings (B01-B20)

| ID | Issue | Severity | Status |
|----|-------|----------|--------|
| B01 | random_vec normalization | Low | Documented |
| B06 | Split position_in_parent random | Low | Visual diff |
| B07 | Node tangent not tracked | Medium | Gap |
| B08 | Virtual nodes not implemented | **High** | Documented (H3/M2) |
| B09 | Origins on straight trunk | Medium | Documented (C1) |
| B11 | propagate_inactive_rec | Low | Gap |
| B17 | get_perpendicular order | Low | Cross product order |

### ManifoldMesher Agent Findings (M20-M33)

| ID | Issue | Severity | Status |
|----|-------|----------|--------|
| M20 | UV child base calculation | Medium | Architectural |
| M21 | UV loops vs per-vertex | Medium | Architectural |
| M28 | Child circle UV index | Medium | Architectural |
| M29 | Child base UV loop indexing | Medium | Architectural |

### Tree Defaults Agent Findings (T01-T22)

| ID | Parameter | C++ | Rust | Severity |
|----|-----------|-----|------|----------|
| T01 | trunk_height | 10.0 | 5.0 | Medium |
| T02 | trunk_radius | 0.3 | 0.5 | Medium |
| T05 | trunk resolution | 3.0/unit | 4 abs | Low |
| T07 | branch length | 9.0 abs | 0.4 ratio | Medium |
| T08 | branch_radius_ratio | 0.4 | 0.3 | Medium |
| T10 | branch_resolution | 3.0 | 0.0 | Medium |
| T11 | branch_flatness | 0.5 | 0.0 | Medium |
| T12 | gravity_strength | 10.0 | 0.0 | Medium |
| T13 | cut_threshold | 0.2 | 0.1 | **High** |
| T14 | flower_threshold | 0.5 | 0.15 | Low |
| T15 | growth_randomness | 0.1 | 0.2 | Low |
| T19 | pipe_radius_min | 0.01 | 0.02 | Low |

### Property System Agent Findings (P01-P09)

| ID | Issue | Severity | Status |
|----|-------|----------|--------|
| P01 | Curve power optimization | Low | Minor perf |
| P02 | RandomProperty unseeded RNG | Medium | Keep Rust |

### CrownShape Agent Findings (CS01-CS08)

All 8 formula shapes verified matching. Rust has 2 additional shapes (Spreading, Umbrella).

### Helper Functions Agent Findings (H10-H16)

| ID | Issue | Severity | Status |
|----|-------|----------|--------|
| H10 | random_vec normalization | Medium | New finding |
| H11 | get_orthogonal cross order | Medium | New finding |
| H12 | lerp_vec3 clamping | Low | Documented M3 |
| H13 | Smoothing adjacency | Low | Documented M9 |
| H14 | get_look_at_rot thresholds | Medium | Partial |

### Smoothing/Foliage Agent Findings (S01-S04, F01-F05)

| ID | Issue | Severity | Status |
|----|-------|----------|--------|
| S01 | Polygon edge skipping | Medium | Architectural |
| F01-F05 | Foliage system | N/A | Rust original |

---

## Test Command

```bash
cd rust && cargo test
# Expected: 73+ tests pass
```
