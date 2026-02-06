# editor_junction_capture.gd
# Attach this to a Node in your scene, then call capture_all() from the editor
# Or run from editor: Tools > Run Script

@tool
extends Node

@export var pixy_tree: Node3D
@export var camera: Camera3D
@export_dir var output_directory: String = "user://junction_analysis"
@export var capture_button: bool = false:
	set(value):
		if value and Engine.is_editor_hint():
			capture_all()

var iteration: int = 0


func _ready() -> void:
	if not Engine.is_editor_hint():
		return
	# Auto-find nodes if not set
	if not pixy_tree:
		pixy_tree = get_tree().get_first_node_in_group("pixy_tree")
		if not pixy_tree:
			pixy_tree = get_parent().get_node_or_null("PixyTree")
	if not camera:
		camera = get_viewport().get_camera_3d()


func capture_all() -> void:
	"""Capture junction-focused screenshots."""
	print("=== Starting Junction Capture ===")

	if not pixy_tree:
		push_error("No PixyTree node set!")
		return
	if not camera:
		push_error("No Camera set!")
		return

	# Create output dir
	DirAccess.make_dir_recursive_absolute(ProjectSettings.globalize_path(output_directory))

	# Get tree params
	var tree_height: float = pixy_tree.get("trunk_height") if pixy_tree.get("trunk_height") else 10.0
	var trunk_radius: float = pixy_tree.get("trunk_radius") if pixy_tree.get("trunk_radius") else 0.3
	var branch_start: float = pixy_tree.get("branch_start") if pixy_tree.get("branch_start") else 0.3

	print("Tree: height=%.2f, radius=%.2f, branch_start=%.2f" % [tree_height, trunk_radius, branch_start])

	# Calculate junction zone
	var junction_height: float = tree_height * branch_start
	var junction_top: float = tree_height * 0.65
	var junction_mid: float = (junction_height + junction_top) / 2.0

	# Camera distances
	var close_dist: float = max(trunk_radius * 10.0, 4.0)
	var med_dist: float = close_dist * 1.6

	var angles: Array[Dictionary] = [
		{
			"name": "junction_closeup_side",
			"position": Vector3(close_dist, junction_height + trunk_radius * 3, 0),
			"look_at": Vector3(0, junction_height + trunk_radius, 0),
		},
		{
			"name": "junction_closeup_34",
			"position": Vector3(close_dist * 0.7, junction_height + trunk_radius * 2, close_dist * 0.7),
			"look_at": Vector3(0, junction_height + trunk_radius, 0),
		},
		{
			"name": "junctions_medium",
			"position": Vector3(med_dist, junction_mid + trunk_radius, med_dist * 0.4),
			"look_at": Vector3(0, junction_mid, 0),
		},
		{
			"name": "junctions_underside",
			"position": Vector3(med_dist * 0.5, junction_height * 0.5, med_dist * 0.5),
			"look_at": Vector3(0, junction_mid * 0.8, 0),
		},
		{
			"name": "upper_junctions",
			"position": Vector3(close_dist * 0.8, junction_top + trunk_radius, close_dist * 0.5),
			"look_at": Vector3(0, junction_top * 0.85, 0),
		},
	]

	var saved_transform: Transform3D = camera.global_transform
	var paths: Array[String] = []

	for angle in angles:
		# Position camera
		camera.global_position = angle["position"]
		camera.look_at(angle["look_at"])

		print("  Capturing %s..." % angle["name"])

		# Force render update
		await get_tree().process_frame
		await get_tree().process_frame
		await RenderingServer.frame_post_draw

		# Capture
		var viewport: Viewport = get_viewport()
		var image: Image = viewport.get_texture().get_image()

		if image == null:
			push_error("Failed to capture %s" % angle["name"])
			continue

		var filename: String = "iter_%03d_%s.png" % [iteration, angle["name"]]
		var path: String = output_directory + "/" + filename
		var global_path: String = ProjectSettings.globalize_path(path)

		var err: Error = image.save_png(global_path)
		if err != OK:
			push_error("Failed to save %s: %d" % [global_path, err])
			continue

		paths.append(global_path)
		print("    Saved: %s" % global_path)

	# Restore camera
	camera.global_transform = saved_transform
	iteration += 1

	print("\n=== Capture Complete ===")
	print("Saved %d images to: %s" % [paths.size(), ProjectSettings.globalize_path(output_directory)])
	print("\nTo analyze, run:")
	print("  python /Users/ladvien/pixy_tree/analyze_junctions.py --dir '%s'" % ProjectSettings.globalize_path(output_directory))
