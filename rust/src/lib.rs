use godot::prelude::*;

mod tree;

struct PixyTreeExtension;

#[gdextension]
unsafe impl ExtensionLibrary for PixyTreeExtension {}
