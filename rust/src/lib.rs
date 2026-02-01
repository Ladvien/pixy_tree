use godot::prelude::*;

mod branch;
mod editor_plugin;
mod tree;

struct PixyTreeExtension;

#[gdextension]
unsafe impl ExtensionLibrary for PixyTreeExtension {}
