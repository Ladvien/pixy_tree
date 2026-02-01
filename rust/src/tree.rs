use godot::prelude::*;

#[derive(GodotClass)]
#[class(base=Node3D)]
pub struct PixyTree {
    base: Base<Node3D>,
}

#[godot_api]
impl INode3D for PixyTree {
    fn init(base: Base<Node3D>) -> Self {
        Self { base }
    }
}

#[godot_api]
impl PixyTree {}
