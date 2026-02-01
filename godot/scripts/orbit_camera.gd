extends Camera3D
## Simple orbit camera for testing - use mouse to rotate, scroll to zoom

@export var target: Vector3 = Vector3.ZERO
@export var distance: float = 30.0
@export var min_distance: float = 5.0
@export var max_distance: float = 100.0
@export var rotation_speed: float = 0.005
@export var zoom_speed: float = 2.0

## 3/4 top-down view settings
@export var frame_yaw: float = PI / 4.0  ## 45 degrees - corner view
@export var frame_pitch: float = -PI / 5.0  ## ~36 degrees down from horizontal
@export var frame_distance_multiplier: float = 1.5  ## Distance relative to tree height

var _yaw: float = 0.0
var _pitch: float = -0.5
var _dragging: bool = false


func _ready() -> void:
	_update_camera_position()


func _input(event: InputEvent) -> void:
	if event is InputEventMouseButton:
		var mb := event as InputEventMouseButton
		if mb.button_index == MOUSE_BUTTON_RIGHT:
			_dragging = mb.pressed
		elif mb.button_index == MOUSE_BUTTON_WHEEL_UP:
			distance = max(min_distance, distance - zoom_speed)
			_update_camera_position()
		elif mb.button_index == MOUSE_BUTTON_WHEEL_DOWN:
			distance = min(max_distance, distance + zoom_speed)
			_update_camera_position()

	elif event is InputEventMouseMotion and _dragging:
		var mm := event as InputEventMouseMotion
		_yaw -= mm.relative.x * rotation_speed
		_pitch -= mm.relative.y * rotation_speed
		_pitch = clamp(_pitch, -PI / 2 + 0.1, PI / 2 - 0.1)
		_update_camera_position()


func _update_camera_position() -> void:
	var offset := Vector3(
		cos(_pitch) * sin(_yaw),
		sin(_pitch),
		cos(_pitch) * cos(_yaw)
	) * distance

	global_position = target + offset
	look_at(target, Vector3.UP)


## Frame the tree with a 3/4 top-down view for easy assessment
func frame_tree(height: float, _radius: float) -> void:
	# Set the target to the vertical center of the tree
	target = Vector3(0.0, height / 2.0, 0.0)

	# Set 3/4 top-down angle
	_yaw = frame_yaw
	_pitch = frame_pitch

	# Calculate distance to fit the tree in view
	distance = clamp(height * frame_distance_multiplier, min_distance, max_distance)

	_update_camera_position()
