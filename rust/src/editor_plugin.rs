use godot::classes::editor_plugin::CustomControlContainer;
use godot::classes::{
    Button, Camera3D, EditorPlugin, IEditorPlugin, InputEvent, InputEventKey, MarginContainer,
    Node3D, VBoxContainer,
};
use godot::global::Key;
use godot::prelude::*;

/// Camera framing constants for 3/4 top-down view
const FRAME_YAW: f32 = std::f32::consts::PI / 4.0; // 45 degrees
const FRAME_PITCH: f32 = -std::f32::consts::PI / 5.0; // ~36 degrees down
const FRAME_DISTANCE_MULTIPLIER: f32 = 3.0;

#[derive(GodotClass)]
#[class(tool, init, base=EditorPlugin)]
pub struct PixyTreePlugin {
    base: Base<EditorPlugin>,
    #[init(val = None)]
    current_tree: Option<Gd<Node>>,
    #[init(val = None)]
    margin_container: Option<Gd<MarginContainer>>,
    #[init(val = None)]
    generate_button: Option<Gd<Button>>,
    #[init(val = None)]
    clear_button: Option<Gd<Button>>,
}

#[godot_api]
impl IEditorPlugin for PixyTreePlugin {
    fn enter_tree(&mut self) {
        // Create MarginContainer
        let mut margin_container = MarginContainer::new_alloc();
        margin_container.set_name("PixyTreeMargin");
        margin_container.set_visible(false);
        margin_container.set_custom_minimum_size(Vector2::new(120.0, 0.0));
        margin_container.add_theme_constant_override("margin_top", 8);
        margin_container.add_theme_constant_override("margin_left", 8);
        margin_container.add_theme_constant_override("margin_right", 8);
        margin_container.add_theme_constant_override("margin_bottom", 8);

        // Create VBoxContainer
        let mut toolbar = VBoxContainer::new_alloc();
        toolbar.set_name("PixyTreeToolbar");
        toolbar.add_theme_constant_override("separation", 8);

        // Create buttons
        let mut generate_button = Button::new_alloc();
        generate_button.set_text("Generate (G)");
        generate_button.set_custom_minimum_size(Vector2::new(100.0, 30.0));

        let mut clear_button = Button::new_alloc();
        clear_button.set_text("Clear (C)");
        clear_button.set_custom_minimum_size(Vector2::new(100.0, 30.0));

        // Add buttons to toolbar
        toolbar.add_child(&generate_button);
        toolbar.add_child(&clear_button);
        margin_container.add_child(&toolbar);

        // Connect signals using self.to_gd() pattern
        let plugin_ref = self.to_gd();
        generate_button.connect(
            "pressed",
            &Callable::from_object_method(&plugin_ref, "on_generate_pressed"),
        );
        clear_button.connect(
            "pressed",
            &Callable::from_object_method(&plugin_ref, "on_clear_pressed"),
        );

        // Add to SPATIAL_EDITOR_SIDE_LEFT
        self.base_mut().add_control_to_container(
            CustomControlContainer::SPATIAL_EDITOR_SIDE_LEFT,
            &margin_container,
        );

        // Store references
        self.margin_container = Some(margin_container);
        self.generate_button = Some(generate_button);
        self.clear_button = Some(clear_button);
    }

    fn exit_tree(&mut self) {
        self.generate_button = None;
        self.clear_button = None;

        if let Some(mut margin) = self.margin_container.take() {
            self.base_mut().remove_control_from_container(
                CustomControlContainer::SPATIAL_EDITOR_SIDE_LEFT,
                &margin,
            );
            margin.queue_free();
        }
    }

    fn handles(&self, object: Gd<Object>) -> bool {
        object.get_class() == "PixyTree"
    }

    fn edit(&mut self, object: Option<Gd<Object>>) {
        if let Some(obj) = object {
            if let Ok(node) = obj.try_cast::<Node>() {
                self.current_tree = Some(node);
                self.set_ui_visible(true);
                return;
            }
        }
        self.set_ui_visible(false);
    }

    fn make_visible(&mut self, visible: bool) {
        self.set_ui_visible(visible);
        if !visible {
            self.current_tree = None;
        }
    }

    fn forward_3d_gui_input(
        &mut self,
        _viewport_camera: Option<Gd<Camera3D>>,
        event: Option<Gd<InputEvent>>,
    ) -> i32 {
        // Only handle input when we have a tree selected
        if self.current_tree.is_none() {
            return 0; // Don't consume
        }

        let Some(event) = event else {
            return 0;
        };

        // Check for key press events
        if let Ok(key_event) = event.try_cast::<InputEventKey>() {
            if key_event.is_pressed() && !key_event.is_echo() {
                match key_event.get_keycode() {
                    Key::G => {
                        self.on_generate_pressed();
                        return 1; // Consume the event
                    }
                    Key::C => {
                        self.on_clear_pressed();
                        return 1; // Consume the event
                    }
                    _ => {}
                }
            }
        }

        0 // Don't consume
    }
}

#[godot_api]
impl PixyTreePlugin {
    #[func]
    fn on_generate_pressed(&mut self) {
        self.call_tree_method("generate");
        self.frame_camera_on_tree();
    }

    #[func]
    fn on_clear_pressed(&mut self) {
        self.call_tree_method("clear");
    }
}

impl PixyTreePlugin {
    fn set_ui_visible(&mut self, visible: bool) {
        if let Some(ref mut margin) = self.margin_container {
            margin.set_visible(visible);
        }
    }

    fn call_tree_method(&mut self, method_name: &str) {
        if let Some(ref tree) = self.current_tree {
            if tree.is_instance_valid() {
                let mut tree_clone = tree.clone();
                if tree_clone.has_method(method_name) {
                    tree_clone.call(method_name, &[]);
                }
            }
        }
    }

    fn frame_camera_on_tree(&mut self) {
        // Get tree dimensions
        let (tree_pos, tree_height) = if let Some(ref tree) = self.current_tree {
            if !tree.is_instance_valid() {
                return;
            }
            let tree_clone = tree.clone();
            if let Ok(node3d) = tree_clone.try_cast::<Node3D>() {
                let pos = node3d.get_global_position();
                // Get trunk_height from the tree
                let height: f32 = tree.clone().get("trunk_height").try_to().unwrap_or(5.0);
                (pos, height)
            } else {
                return;
            }
        } else {
            return;
        };

        // Get the editor's 3D viewport camera
        let Some(editor_interface) = self.base_mut().get_editor_interface() else {
            return;
        };
        let Some(viewport) = editor_interface.get_editor_viewport_3d() else {
            return;
        };
        let Some(mut camera) = viewport.get_camera_3d() else {
            return;
        };

        // Calculate target position (center of tree)
        let target = tree_pos + Vector3::new(0.0, tree_height / 2.0, 0.0);

        // Calculate camera position for 3/4 top-down view
        let distance = (tree_height * FRAME_DISTANCE_MULTIPLIER).max(5.0);
        let offset = Vector3::new(
            FRAME_PITCH.cos() * FRAME_YAW.sin(),
            -FRAME_PITCH.sin(),
            FRAME_PITCH.cos() * FRAME_YAW.cos(),
        ) * distance;

        let camera_pos = target + offset;
        camera.set_global_position(camera_pos);
        camera.look_at(target);
    }
}
