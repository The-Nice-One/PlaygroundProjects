--[[pod_format="raw",created="2025-07-30 22:25:00",modified="2025-08-11 18:06:50",revision=278]]
include("utilities.lua")

include("player.lua")
include("projectiles.lua")
include("debug.lua")

include("drops.lua")
include("enemies.lua")

include("health_bars.lua")
include("collisions.lua")

include("ui.lua")

function _init()
	mouse_x, mouse_y, mouse_b = mouse()
	mouse_p, mouse_lock = false, false
	init_player()
	init_projectiles()
	init_debug()
	
	init_drops()
	init_enemies()
	
	init_ui()
	
	-- Some defined constants :)
	degree = 1/360
	
	-- Main game info
	x = 100
	y = 100
	coins = 275
	score = 0
end

function draw_map()
	local current_x = 0
	local current_y = 0
	
	for current_x = 0, 480, 16 do
	   for current_y = 0, 270, 16 do
			spr(2, current_x, current_y)
		end
	end
end

function update_mouse_p()
	if (mouse_b == 0x1 and not mouse_p and not mouse_lock) then
		mouse_lock = true
		mouse_p = true
	elseif (mouse_b == 0x1 and mouse_p) then mouse_p = false
	elseif (mouse_b != 0x1 and mouse_lock) then mouse_lock = false end
end

function _update()
	mouse_x, mouse_y, mouse_b = mouse()
	update_mouse_p()
	
	if (Player.health > 0) then
		update_enemy_spawns()
		update_enemies()
		update_projectiles()
		update_drops()
		update_player_angle()
		update_player_position()
		update_player_attack()
		update_ui()
	else
		Ui.tutorial_stage = 5
	end
	
	update_debug()
end

function _draw()
	cls()
	draw_map()
	draw_drops()
	draw_enemies()
	draw_projectiles()
	draw_player()
	draw_ui()
	draw_debug()
end