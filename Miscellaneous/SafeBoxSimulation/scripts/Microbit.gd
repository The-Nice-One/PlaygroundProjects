extends Control

signal open_door
signal close_door

const frames = {
	"0": [
		0, 1, 1, 0, 0,
		1, 0, 0, 1, 0,
		1, 0, 0, 1, 0,
		1, 0, 0, 1, 0,
		0, 1, 1, 0, 0
	],
	"1": [
		0, 0, 1, 0, 0,
		0, 1, 1, 0, 0,
		0, 0, 1, 0, 0,
		0, 0, 1, 0, 0,
		0, 1, 1, 1, 0
	],
	"2": [
		1, 1, 1, 0, 0,
		0, 0, 0, 1, 0,
		0, 1, 1, 0, 0,
		1, 0, 0, 0, 0,
		0, 1, 1, 1, 0
	],
	"3": [
		1, 1, 1, 1, 0,
		0, 0, 0, 1, 0,
		0, 0, 1, 0, 0,
		1, 0, 0, 1, 0,
		0, 1, 1, 0, 0
	],
	"4": [
		0, 0, 1, 1, 0,
		0, 1, 0, 1, 0,
		1, 0, 0, 1, 0,
		1, 1, 1, 1, 1,
		0, 0, 0, 1, 0
	],
	"5": [
		1, 1, 1, 1, 1,
		1, 0, 0, 0, 0,
		1, 1, 1, 1, 0,
		0, 0, 0, 0, 1,
		1, 1, 1, 1, 0
	],
	"6": [
		0, 0, 0, 1, 0,
		0, 0, 1, 0, 0,
		0, 1, 1, 1, 0,
		1, 0, 0, 0, 1,
		0, 1, 1, 1, 0
	],
	"7": [
		1, 1, 1, 1, 1,
		0, 0, 0, 1, 0,
		0, 0, 1, 0, 0,
		0, 1, 0, 0, 0,
		1, 0, 0, 0, 0
	],
	"8": [
		0, 1, 1, 1, 0,
		1, 0, 0, 0, 1,
		0, 1, 1, 1, 0,
		1, 0, 0, 0, 1,
		0, 1, 1, 1, 0
	],
	"9": [
		0, 1, 1, 1, 0,
		1, 0, 0, 0, 1,
		0, 1, 1, 1, 0,
		0, 0, 1, 0, 0,
		0, 1, 0, 0, 0
	],
	"check": [
		0, 0, 0, 0, 0,
		0, 0, 0, 0, 1,
		0, 0, 0, 1, 0,
		1, 0, 1, 0, 0,
		0, 1, 0, 0, 0
	],
	"cross": [
		0, 0, 0, 0, 0,
		0, 1, 0, 1, 0,
		0, 0, 1, 0, 0,
		0, 1, 0, 1, 0,
		0, 0, 0, 0, 0
	],
	"not available": [
		0, 0, 0, 0, 0,
		0, 0, 0, 1, 0,
		0, 0, 1, 0, 0,
		0, 1, 0, 0, 0,
		0, 0, 0, 0, 0
	]
}

# Small oopsie when importing the led states tilesheet and creating the tiles. :D
func reverse_ids(i):
	if i == 0: return 1
	if i == 1: return 0

# Function for displaying a 5x5 frame on the Microbit's LED display.
func render_frame(id: String):
	var x = 0
	var y = 0
	for i in frames[id]:
		$LEDDisplay.set_cell(x, y, reverse_ids(i))
		x += 1
		if x > 4:
			x = 0
			y += 1

enum Mode {
	EnterPasscode,
	ChangePasscode
}
var mode = Mode.EnterPasscode
var current_number = 0
var pause_input = false

var passcode = "000"
var buffer = ""

var alarm_on = false
var door_open = false

func _process(_delta):
	if alarm_on:
		$SFX/Alarm.stream_paused = false
	else:
		$SFX/Alarm.stream_paused = true

	if not pause_input:
		render_frame(str(current_number))

# Allows the buttons on the microbits to switch the number currently selected.
func _on_ButtonA_pressed():
	if current_number > 0:
		current_number -= 1

func _on_ButtonB_pressed():
	if current_number < 9:
		current_number += 1

func _on_ButtonAAndB_pressed():
	if mode == Mode.EnterPasscode:
		buffer += str(current_number)
		if len(buffer) == 3:
			if buffer == passcode:
				if not alarm_on:
					$SFX/Success.play()
					buffer = ""
					pause_input = true
					render_frame("check")
					yield(get_tree().create_timer(4), "timeout")
					pause_input = false
					door_open = true
					emit_signal("open_door")
				else:
					alarm_on = false
					buffer = ""
					pause_input = true
					render_frame("check")
					yield(get_tree().create_timer(2), "timeout")
					pause_input = false
			else:
				buffer = ""
				$SFX/Closed.play()
				pause_input = true
				render_frame("cross")
				yield(get_tree().create_timer(4), "timeout")
				pause_input = false
	if mode == Mode.ChangePasscode:
		passcode += str(current_number)
		if len(passcode) == 3:
			mode = Mode.EnterPasscode
			pause_input = true
			$SFX/Action.play()
			render_frame("check")
			yield(get_tree().create_timer(2), "timeout")
			pause_input = false


func _on_Main_alarm_on():
	alarm_on = true

# Stores the time when the head button was pressed down.
var time_when_pushed_down = 0
func _on_ButtonHead_button_down():
	time_when_pushed_down = OS.get_ticks_msec()


func _on_ButtonHead_button_up():
	# Checks if the door is open, and if not returns early-preventing you from
	# accessing admin controls or closing the door.
	if not door_open:
		$SFX/NotAvailable.play()
		pause_input = true
		render_frame("not available")
		yield(get_tree().create_timer(2), "timeout")
		pause_input = false
		return

	# Calculates the length of the head button press
	var duration_pressed = (OS.get_ticks_msec() - time_when_pushed_down) / 1000.0
	# If the press is short then the door of the SafeBox will close.
	if duration_pressed < 0.5:
		$SFX/Closed.play()
		pause_input = true
		render_frame("check")
		yield(get_tree().create_timer(4), "timeout")
		pause_input = false
		door_open = false
		emit_signal("close_door")
	# Otherwise, some setup code is ran which will allow you to change the passcode.
	else:
		passcode = ""
		buffer = ""
		current_number = 0
		mode = Mode.ChangePasscode
		pause_input = true
		$SFX/Action.play()
		render_frame("check")
		yield(get_tree().create_timer(2), "timeout")
		pause_input = false
