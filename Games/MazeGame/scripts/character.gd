extends KinematicBody2D
const PLAYER_SPEED = 5
var enlightened = false
var moved = false
var time = 0
var timer_triggered = false
var done = false

func _process(delta):
	if done:
		return
	
	var movement: Vector2 = Vector2(0,0)
	var sprint = false

	if Input.is_action_pressed("Sprint") && $"../CanvasLayer/SprintBar".value >= 1:
		$"../CanvasLayer/SprintBar".value -= 1
		sprint = true
		$"../CanvasLayer/SprintButton".pressed = true
	else:
		$"../CanvasLayer/SprintButton".pressed = false
	
	var speed = (PLAYER_SPEED*(int(sprint)+1))
	movement = Vector2($"../CanvasLayer/joystick".dir.x * speed, $"../CanvasLayer/joystick".dir.y * speed) 
	
	if movement == Vector2(0,0):
		movement.x = (
			int(
				Input.is_action_pressed("Right")||$"../CanvasLayer/Position2D/Right".pressed)
				-int(Input.is_action_pressed("Left")||$"../CanvasLayer/Position2D/Left".pressed)
			)*speed
		movement.y = (
			int(
				Input.is_action_pressed("Down")||$"../CanvasLayer/Position2D/Down".pressed)
				-int(Input.is_action_pressed("Up")||$"../CanvasLayer/Position2D/Up".pressed)
			)*speed
	
	if movement != Vector2(0,0): 
		moved = true
		if !timer_triggered:
			$"../GUI".play("start_timer")
			timer_triggered = true
			yield($"../GUI", "animation_finished")
	
	var collide = move_and_collide(movement)
	
	if collide && (position.x >= 3074 && position.x <= 3123) && floor(position.y) == 1413:
		position.x = 3100
		position.y = 1510
		$"../CanvasLayer/CompleteText".text = "Maze Complete in " + $"../CanvasLayer/Label".text +"!"
		$"../GUI".play("end")
		done = true
	elif collide && not enlightened:
		enlightened = true
		$"../GLOW".play("glow")
		yield($"../GLOW", "animation_finished")
		$"../GLOW".play_backwards("glow")
		yield($"../GLOW", "animation_finished")
		enlightened = false


func _on_Timer_timeout():
	$"../CanvasLayer/SprintBar".value += 1
	if moved:
		time += 0.1
		$"../CanvasLayer/Label".text = str(time) + " seconds"


func _on_SprintButton_button_down():
	Input.action_press("Sprint")

func _on_SprintButton_button_up():
	Input.action_release("Sprint")


func _on_Menu_tab_changed(tab):
	if tab == 0:
		$"../CanvasLayer/SprintButton".hide()
		$"../CanvasLayer/Position2D".hide()
		$"../CanvasLayer/joystick".position.x = 3000
	elif tab == 1:
		$"../CanvasLayer/SprintButton".show()
		$"../CanvasLayer/Position2D".hide()
		$"../CanvasLayer/joystick".position.x = 119
	elif tab == 2:
		$"../CanvasLayer/SprintButton".show()
		$"../CanvasLayer/Position2D".show()
		$"../CanvasLayer/joystick".position.x = 3000
