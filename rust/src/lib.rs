use godot::prelude::*;

mod branch;
mod crown_shape;
mod editor_plugin;
mod foliage;
mod growth;
#[allow(clippy::too_many_arguments, clippy::only_used_in_recursion)]
mod manifold_mesher;
mod property;
mod smoothing;
mod tree;
mod tree_preset;

struct PixyTreeExtension;

#[gdextension]
unsafe impl ExtensionLibrary for PixyTreeExtension {}
