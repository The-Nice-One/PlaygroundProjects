extends Spatial

signal alarm_on

func _on_ButtonOpen_pressed():
	$MainUI.hide()
	$Microbit.show()

func _on_ButtonClose_pressed():
	$MainUI.show()
	$Microbit.hide()

func _on_ButtonMove_pressed():
	$SafeBox.apply_impulse(Vector3(6, 6, 0), Vector3(6, 6, 0))
	yield(get_tree().create_timer(1), "timeout")
	emit_signal("alarm_on")

func _on_Microbit_open_door():
	$MainUI.show()
	$Microbit.hide()
	yield(get_tree().create_timer(1), "timeout")
	$SafeBox/AnimationPlayer.play("opendoor")
	$SafeBox/LED/OmniLight.light_energy = 1.0

func _on_Microbit_close_door():
	$MainUI.show()
	$Microbit.hide()
	yield(get_tree().create_timer(1), "timeout")
	$SafeBox/AnimationPlayer.play_backwards("opendoor")
	$SafeBox/LED/OmniLight.light_energy = 0.0
