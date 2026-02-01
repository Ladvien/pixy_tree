# Pixy Tree Architecture

## Overview

Pixy Tree is a procedural tree generation system for Godot 4, implemented in Rust via GDExtension.

## Core Components

### PixyTree Node
The main entry point, extending `Node3D`. Responsible for:
- Tree parameter configuration (species, size, branching patterns)
- Mesh generation and management
- LOD (Level of Detail) handling

## Planned Features

- [ ] Basic trunk generation
- [ ] Branching algorithms (L-systems, space colonization)
- [ ] Foliage generation
- [ ] Stylized/pixel-art friendly output
- [ ] Multiple tree species presets
- [ ] Runtime LOD support

## Design Principles

1. **Performance**: Heavy computation in Rust, minimal GDScript overhead
2. **Flexibility**: Exposed parameters for diverse tree styles
3. **Integration**: Seamless Godot editor experience with tool scripts
