extends SceneTree

## Capture script for preset evaluation.
## Run with: godot --path godot --windowed --resolution 1024x768 -s res://scripts/preset_capture.gd
## Set PRESET_NAME env var to choose preset (default: Oak)
## Set CAPTURE_DIR env var to choose output directory

var output_path: String = "/private/tmp/claude-501/-Users-ladvien-pixy-tree/preset_capture/"


func _init() -> void:
	var preset_name: String = OS.get_environment("PRESET_NAME")
	if preset_name == "":
		preset_name = "Oak"

	var capture_dir: String = OS.get_environment("CAPTURE_DIR")
	if capture_dir != "":
		output_path = capture_dir

	print("Preset Capture: starting with preset '%s'..." % preset_name)

	DirAccess.make_dir_recursive_absolute(output_path)

	var scene: Node3D = load("res://scenes/test_scene.tscn").instantiate() as Node3D
	root.add_child(scene)

	var pixy_tree: Node3D = scene.get_node_or_null("PixyTree")
	if not pixy_tree:
		push_error("PixyTree not found")
		quit(1)
		return

	# Let the orbit camera exist but we'll override it later

	# Apply preset
	var preset_map: Dictionary = {
		"Custom": 0, "Oak": 1, "Pine": 2, "Willow": 3, "Birch": 4,
		"Palm": 5, "Cypress": 6, "Bonsai": 7, "Maple": 8, "Spruce": 9,
		"Poplar": 10, "Baobab": 11, "DragonTree": 12, "JapaneseMaple": 13,
		"DeadTree": 14, "Redwood": 15, "Elm": 16, "Fir": 17, "Cedar": 18,
		"JoshuaTree": 19, "Olive": 20, "CherryBlossom": 21, "Acacia": 22,
		"Beech": 23, "Ginkgo": 24, "WeepingCherry": 25, "WorldTree": 26,
		"CrystalTree": 27, "CorruptedTree": 28, "GlowingTree": 29,
		"BlueSpruce": 30, "DouglasFir": 31, "PonderosaPine": 32,
		"BristleconePine": 33, "Ash": 34, "Linden": 35, "Sycamore": 36,
		"Aspen": 37, "RoyalPalm": 38, "FanPalm": 39, "Eucalyptus": 40,
	}

	var preset_id: int = preset_map.get(preset_name, 3)
	pixy_tree.set("preset", preset_id)

	# Apply any override params from environment
	var params_json: String = OS.get_environment("TREE_PARAMS")
	if params_json != "":
		var json := JSON.new()
		var err := json.parse(params_json)
		if err == OK:
			var params: Dictionary = json.data
			for key in params:
				pixy_tree.set(key, params[key])
			print("Preset Capture: applied %d param overrides" % params.size())

	# Generate
	pixy_tree.call("generate")
	print("Preset Capture: tree generated, waiting for render...")

	await wait_frames(5)

	# Create a fresh camera with no scripts attached
	var camera := Camera3D.new()
	camera.fov = 50.0  # Narrower FOV for less distortion
	scene.add_child(camera)
	camera.current = true

	await wait_frames(5)

	# Calculate tree extents
	var tree_height: float = pixy_tree.get("trunk_height")
	var branch_length: float = pixy_tree.get("branch_length")
	var branch_angle: float = pixy_tree.get("branch_angle")
	var angle_rad: float = deg_to_rad(branch_angle)

	# Approximate crown dimensions
	var crown_width: float = branch_length * sin(angle_rad) * 2.0
	var crown_top: float = tree_height + branch_length * cos(angle_rad)
	var total_h: float = crown_top
	var total_w: float = max(crown_width, tree_height * 0.8)

	# Distance to fit tree in FOV with margin
	var fov_rad: float = deg_to_rad(camera.fov / 2.0)
	var fit_dist_v: float = (total_h * 0.6) / tan(fov_rad)  # vertical fit
	var aspect: float = 1024.0 / 768.0
	var fit_dist_h: float = (total_w * 0.6) / tan(fov_rad * aspect)  # horizontal fit
	var cam_dist: float = max(fit_dist_v, fit_dist_h, 25.0)

	var center_y: float = total_h * 0.45

	print("Preset Capture: tree_h=%.1f crown_w=%.1f crown_top=%.1f cam_dist=%.1f center_y=%.1f" % [tree_height, crown_width, crown_top, cam_dist, center_y])

	# Capture views
	var views: Array[Dictionary] = [
		{
			"name": "full_front",
			"pos": Vector3(cam_dist * 0.7, center_y + 1.0, cam_dist * 0.7),
			"look": Vector3(0, center_y, 0),
		},
		{
			"name": "full_side",
			"pos": Vector3(cam_dist, center_y * 0.8, 0),
			"look": Vector3(0, center_y, 0),
		},
		{
			"name": "detail_junction",
			"pos": Vector3(cam_dist * 0.35, tree_height * 0.5, cam_dist * 0.35),
			"look": Vector3(0, tree_height * 0.4, 0),
		},
	]

	for view in views:
		camera.global_position = view["pos"]
		camera.look_at(view["look"])

		print("Preset Capture: view=%s pos=%s" % [view["name"], str(camera.global_position)])

		await wait_frames(10)
		await RenderingServer.frame_post_draw

		var image: Image = root.get_viewport().get_texture().get_image()
		if image == null:
			push_error("Failed to capture " + view["name"])
			continue

		var filepath: String = output_path + preset_name.to_lower() + "_" + str(view["name"]) + ".png"
		var err_save: Error = image.save_png(filepath)
		if err_save == OK:
			print("Preset Capture: saved " + filepath)
		else:
			push_error("Failed to save " + filepath + ": " + str(err_save))

	print("Preset Capture: done")
	quit(0)


func wait_frames(count: int) -> void:
	for i in range(count):
		await process_frame
