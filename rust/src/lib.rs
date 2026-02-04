use godot::prelude::*;

mod branch;
mod crown_shape;
mod editor_plugin;
mod foliage;
mod growth;
mod property;
mod smoothing;
mod tree;
mod tree_preset;

struct PixyTreeExtension;

#[gdextension]
unsafe impl ExtensionLibrary for PixyTreeExtension {}
