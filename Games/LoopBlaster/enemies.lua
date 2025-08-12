--[[pod_format="raw",created="2025-07-31 21:24:54",modified="2025-08-11 18:09:51",revision=101]]
function init_enemies()
	enemies = {}
	enemies_started = t()
	enemies_spawn_rate = 14
	enemies_spawn_last_time = 0
	
	enemies_spawn_time = 1
	enemies_spawn_modifier = 0
	enemies_health_modifier = 0
end

function update_enemy_spawns()
	if (Ui.tutorial_stage != 4) then 
		enemies_started = t()
		enemies_spawn_last_time = t() - enemies_spawn_rate / 2
	end
	
	enemies_spawn_rate = 14 - (sqrt(t() - enemies_started)/1.9)
	if (enemies_spawn_rate < 3.8) enemies_spawn_rate = 3.8
	
	enemies_health_modifier = math.log(score+1)^2
	enemies_spawn_modifier = (math.log(score+1)^2)/14
	
	enemies_spawn_time = 1 - (sqrt(score+1)/100)
	if (enemies_spawn_time < 0.1) enemies_spawn_time = 0.1
	

	if ( (t() - enemies_spawn_last_time) >= enemies_spawn_rate - rnd(enemies_spawn_modifier) ) then
		enemies_spawn_last_time = t()
		spawn_enemy(flr(rnd(480)), flr(rnd(270)), 4, 3+ceil(rnd(enemies_health_modifier)), enemies_spawn_time)
	end
end

function spawn_enemy(x, y, radius, health, spawn_time)
	local radius = radius+flr((health)/6)
	enemies[#enemies+1] = {
		x = x,
		y = y,
		radius = radius,
		
		health = health,
		max_health = health,
		
		spawn_time = t()+spawn_time,
		spawned = false,
		spawn_rnd = rnd(100),
		
		collider = {
			left = x - radius,
			right = x + radius,
			top = y - radius,
			bottom = y + radius
		}
	}
end

function update_enemies()
	for i = 1, #enemies do
		if (not enemies[i].spawned) then
			if (t() >= enemies[i].spawn_time) enemies[i].spawned = true
		else
			slope_x = x - enemies[i].x
			slope_y = y - enemies[i].y
		
			magnitude = sqrt(slope_x^2 + slope_y^2)
		
			slope_x /= magnitude
			slope_y /= magnitude
		
			enemies[i].x += slope_x
			enemies[i].y += slope_y
		
			enemies[i].collider.left += slope_x
			enemies[i].collider.right += slope_x
			enemies[i].collider.top += slope_y
			enemies[i].collider.bottom += slope_y
		end
	end
end

function damage_enemy(index, damage)
	enemies[index].health -= damage
	if (enemies[index].health <= 0) then
		score += enemies[index].max_health
		spawn_drop(enemies[index].x, enemies[index].y, enemies[index].max_health)
		deli(enemies, index)
	end
end

function draw_enemies()
	for i = 1, #enemies do
		if (enemies[i].spawned) then
			circfill(enemies[i].x, enemies[i].y, enemies[i].radius, 2)
			circ(enemies[i].x, enemies[i].y, enemies[i].radius, 8)
			draw_bounded_collider(enemies[i].collider)
		
			if (enemies[i].health != enemies[i].max_health) then 
				draw_bar(enemies[i].x, 
					enemies[i].y-(enemies[i].radius-4), 
					enemies[i].radius*2+1,
					3,
					enemies[i].health,
					enemies[i].max_health, "red")
			end
		else
			circfill(enemies[i].x, enemies[i].y, enemies[i].radius, 2)
		end
	end
end