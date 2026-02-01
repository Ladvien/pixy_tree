# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Critical Info
- Use Godot 4.6
- Always consult documentation, download it if needed
- Prefer Godot `export` variables over magic numbers or constants

## Docs
- Godot Rust Book - https://godot-rust.github.io/book/
- Godot API Docs - https://godot-rust.github.io/docs/ and https://godot-rust.github.io/docs/gdext/master/godot/
- Godot Github - https://github.com/godot-rust/gdext

## Project Overview

Pixy Tree is a Godot 4 procedural tree generation tool written in Rust using GDExtension (godot-rust/gdext). It creates stylized trees for 3D pixel art games.

## Build Commands

```bash
# Build the Rust GDExtension library
cd rust && cargo build

# Build release version
cd rust && cargo build --release

# Run tests
cd rust && cargo test

# Check code without building
cd rust && cargo check

# Format code
cd rust && cargo fmt

# Lint code
cd rust && cargo clippy
```

## Project Structure

```
pixy_tree/
├── godot/                        # Godot project files
│   ├── project.godot             # Godot project configuration
│   ├── pixy_tree.gdextension     # GDExtension configuration
│   ├── scenes/
│   │   └── test_scene.tscn       # Test scene with tree node
│   └── scripts/
│       └── orbit_camera.gd       # Camera controller for testing
└── rust/                         # Rust GDExtension library
    ├── Cargo.toml
    └── src/
        ├── lib.rs                # Extension entry point
        └── tree.rs               # PixyTree node implementation
```

## Architecture

- **GDExtension Entry**: `rust/src/lib.rs` - Registers the extension with Godot via `#[gdextension]` macro
- **PixyTree Node**: `rust/src/tree.rs` - Main tree generation node (extends Node3D)
- **Library Output**: Compiled to `rust/target/{debug,release}/libpixy_tree.dylib` (macOS), `.so` (Linux), or `.dll` (Windows)
- **Godot Integration**: The `.gdextension` file in `godot/` tells Godot where to find the compiled library

## Development Workflow

1. Make changes to Rust code in `rust/src/`
2. Run `cargo build` from `rust/` directory
3. Open Godot project from `godot/` directory - it auto-loads the extension
4. Godot 4.2+ supports hot reloading - changes rebuild without restarting Godot

## Key Dependencies

- `godot` crate from godot-rust/gdext (master branch) - Rust bindings for Godot 4
- `noise` crate - For procedural generation algorithms
- `rayon` / `bevy_tasks` - For parallel processing

## GDExtension Notes

- Custom Godot classes are created using `#[derive(GodotClass)]`
- Methods exposed to GDScript use `#[func]`
- Signals use `#[signal]`
- See https://godot-rust.github.io/book/ for gdext documentation
