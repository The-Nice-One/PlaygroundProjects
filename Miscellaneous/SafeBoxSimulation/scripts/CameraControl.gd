extends Position3D

func _process(delta):
	if Input.is_action_pressed("left"):
		rotation.y -= 1*delta
	if Input.is_action_pressed("right"):
		rotation.y += 1*delta
