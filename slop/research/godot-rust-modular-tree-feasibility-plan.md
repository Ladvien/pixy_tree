# Porting Modular Tree to Godot 4.6 GDExtension Rust

Creating a procedural tree generator plugin for Godot using Rust is **technically feasible and recommended**. The modular_tree Blender addon uses a recursive TreeFunction model—not L-systems or space colonization—that translates well to Rust's ownership system. An existing C++ implementation (Tree3D) already proves GDExtension viability for this exact use case.

## What modular_tree actually does under the hood

The modular_tree addon uses a **hierarchical function execution pattern** rather than classical L-systems. At its core, `TreeFunction` objects execute recursively: a trunk function creates the base structure, then calls child branch functions that operate on the trunk's output. Each branch function can itself have children for secondary branches. The `ManifoldMesher` converts this skeleton into smooth, properly-connected mesh geometry.

Key parameters fall into three categories: **structural** (trunk height, branch length, radius decrease rate), **behavioral** (split probability, gravity strength, up-attraction), and **randomization** (twist, spin randomness, pruning strength). The mesh pipeline generates edge loops perpendicular to branch direction at configurable intervals, connecting them with quad-dominant topology. UVs use cylindrical projection along branches, and resolution scales with branch radius—thicker branches get more subdivisions.

This architecture maps elegantly to Rust traits. A `TreeFunction` trait with an `execute(&mut TreeState)` method, combined with `Box<dyn TreeFunction>` children, recreates the hierarchical model while leveraging Rust's type system for safety.

## GDExt delivers full mesh generation capability

Godot-rust's gdext library provides complete access to Godot's mesh APIs. `ArrayMesh::new_gd()` creates meshes directly from packed arrays—`PackedVector3Array` for vertices and normals, `PackedVector2Array` for UVs, `PackedInt32Array` for indices. These arrays support direct slice access from Rust (`vertices.as_mut_slice()`) and iterator construction, making data transfer efficient.

`SurfaceTool` offers a higher-level builder pattern with automatic normal and tangent generation:

```rust
let mut st = SurfaceTool::new_gd();
st.begin(PrimitiveType::TRIANGLES);
st.set_normal(Vector3::new(0.0, 1.0, 0.0));
st.set_uv(Vector2::new(0.0, 0.0));
st.add_vertex(Vector3::new(0.0, 0.0, 0.0));
let mesh = st.commit();
```

Editor integration works through `#[class(tool, init, base=EditorPlugin)]` attributes. Properties exposed via `#[export]` appear in the inspector with full hint support—ranges, enums, and custom groups. Critically, **tool scripts run in the editor**, enabling real-time preview through custom setters that trigger mesh regeneration when parameters change.

## The Rust ecosystem provides essential building blocks

**For L-system logic** (useful for stylized pixel-art trees), `dcc-lsystem` offers a complete implementation with stochastic rules and push/pop stack operations. Its 2D turtle renderer is irrelevant—you'd write a custom 3D interpreter—but the grammar engine is production-ready. For the hierarchical TreeFunction approach, a custom implementation is straightforward.

**For variation and organic feel**, `fastnoise-lite` delivers excellent performance with OpenSimplex2, Perlin, and cellular noise. The `noise` crate offers more variety (fractals, turbulence, combinators) at slight performance cost. Both integrate cleanly with Godot's coordinate system.

**For math operations**, `glam` is the clear choice. Godot-rust internally uses glam, and conversion methods (`Vector3::to_glam()`, `Vector3::from_glam()`) exist in the API. The f32 precision matches Godot's defaults, and SIMD acceleration comes free on x86_64 and ARM.

No Godot-compatible mesh generation crate exists—you'll build this layer custom, which actually provides more control for the pixel-art aesthetic.

## Technical risks are manageable with proper architecture

**Threading requires careful handling.** Godot classes aren't thread-safe, and gdext needs the `experimental-threads` feature for certain editor scenarios. The solution: run all tree generation algorithms in pure Rust threads (using Rayon for parallelism), then marshal completed mesh data to the main thread for upload via `call_deferred`. This pattern, used by Voxel Tools, isolates the performance-critical work from Godot's threading model.

**The mesh upload is the true bottleneck**, not generation. For very large trees, consider progressive generation or LOD systems. The algorithmic work in pure Rust will be extremely fast—benchmarks show Rust approximately **50% faster** than GDScript for high-iteration scenarios, and mesh generation is CPU-bound computation.

**gdext occasionally introduces breaking changes** between versions. Pin your dependency version and budget time for migrations when updating. The library provides migration guides for each release.

## Recommended architecture and module structure

```
gdext_modular_tree/
├── Cargo.toml
├── src/
│   ├── lib.rs                  # GDExtension entry point
│   ├── tree/
│   │   ├── mod.rs
│   │   ├── function.rs         # TreeFunction trait + implementations
│   │   ├── trunk.rs            # TrunkFunction
│   │   ├── branch.rs           # BranchFunction
│   │   ├── grow.rs             # GrowFunction
│   │   └── state.rs            # TreeState, TreeNode structures
│   ├── mesh/
│   │   ├── mod.rs
│   │   ├── mesher.rs           # ManifoldMesher equivalent
│   │   ├── edge_loop.rs        # Edge loop generation
│   │   └── junction.rs         # Branch junction topology
│   ├── pixel_art/
│   │   ├── mod.rs
│   │   ├── quantizer.rs        # Vertex snapping to grid
│   │   └── palette.rs          # Vertex color mapping
│   ├── noise/
│   │   └── mod.rs              # Noise wrapper for variation
│   ├── godot/
│   │   ├── mod.rs
│   │   ├── tree_generator.rs   # Main GodotClass node
│   │   ├── editor_plugin.rs    # EditorPlugin integration
│   │   └── inspector.rs        # Custom inspector (optional)
│   └── export/
│       └── properties.rs       # Exported property definitions
```

The core `TreeFunction` trait abstracts the generation pattern:

```rust
pub trait TreeFunction: Send + Sync {
    fn execute(&self, state: &mut TreeState, params: &TreeParams);
    fn add_child(&mut self, child: Box<dyn TreeFunction>);
}
```

The main Godot node exposes parameters and triggers generation:

```rust
#[derive(GodotClass)]
#[class(tool, init, base=MeshInstance3D)]
pub struct ProceduralTree {
    base: Base<MeshInstance3D>,
    
    #[export(range = (0.1, 10.0, 0.1))]
    trunk_height: f32,
    
    #[export(range = (0.05, 2.0, 0.01))]
    trunk_radius: f32,
    
    #[export(range = (0.0, 1.0, 0.01))]
    split_probability: f32,
    
    #[export(range = (1, 100))]
    seed: i32,
    
    // Pixel art specific
    #[export(range = (0.0, 1.0, 0.0625))]
    vertex_snap: f32,
}
```

## Pixel-art specific adaptations

The modular_tree's smooth organic output needs modification for 3D pixel art. Implement a **vertex quantizer** that snaps positions to a configurable grid (`vertex_snap` parameter). Reduce `radial_resolution` to 4-8 vertices per cross-section for angular, low-poly silhouettes. Replace smooth normals with flat shading by duplicating vertices at face boundaries.

For color, use vertex colors mapped to a limited palette rather than UV-based textures. The `pixel_art/palette.rs` module would map trunk radius or height to palette indices, creating banded color zones characteristic of pixel art aesthetics.

Consider generating separate meshes for trunk, branches, and leaves (like modular_tree's Final mode) so each can have distinct materials and LOD behavior.

## Required dependencies with versions

```toml
[dependencies]
godot = { git = "https://github.com/godot-rust/gdext", branch = "master" }
glam = "0.31"
fastnoise-lite = "1.1"
rayon = "1.10"

[features]
default = []
experimental-threads = ["godot/experimental-threads"]

[lib]
crate-type = ["cdylib"]
```

Pin godot-rust to a specific commit for stability once development begins. The `experimental-threads` feature should be enabled if background generation is needed.

## Development phases and timeline

**Phase 1 (2-3 weeks): Core generation engine.** Implement TreeFunction trait, TrunkFunction, basic BranchFunction. Build mesh generation with edge loops and simple junctions. Output to ArrayMesh without editor integration. Validate mesh correctness in Godot.

**Phase 2 (2 weeks): Godot integration.** Create ProceduralTree node with exported parameters. Implement `#[class(tool)]` for editor execution. Add property change detection for live preview. Build basic inspector groups.

**Phase 3 (2 weeks): Advanced features.** Implement GrowFunction with full parameter set. Add noise-based variation. Build proper branch junction topology (ManifoldMesher equivalent). Implement split logic with pruning and shape factors.

**Phase 4 (1-2 weeks): Pixel art adaptation.** Vertex quantization system. Flat shading mode. Vertex color palette support. LOD generation for multiple detail levels.

**Phase 5 (1 week): Polish and optimization.** Background thread generation with progress callback. Performance profiling and optimization. Documentation and example scenes.

## Feasibility verdict: proceed with confidence

The technical path is clear. gdext provides the necessary APIs, Rust crates cover the algorithmic needs, and Tree3D proves GDExtension works for exactly this use case. The main implementation effort lies in translating modular_tree's TreeFunction pattern and ManifoldMesher to idiomatic Rust—approximately **6-10 weeks** for a production-quality plugin with pixel-art features.

Start by implementing the simplest possible tree (trunk-only) end-to-end through the entire pipeline. This validates the architecture before investing in complex branch logic. The pixel-art adaptations layer cleanly on top once core generation works.
