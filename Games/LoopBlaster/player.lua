--[[pod_format="raw",created="2025-07-31 21:20:55",modified="2025-08-11 18:07:24",revision=110]]
function init_player()
	Player = {}
	Player.sides = 3
	Player.points = {}
	Player.radius = 8
	
	Player.speed = 3
	Player.projectile_damage = 1
	Player.projectile_speed = 3
	Player.projectile_pierce = 1
	
	Player.attack_speed = 1
	Player.attack_last_time = -1
	
	Player.attack_sides = {1}
	Player.attacks_per_fire = 1
	
	Player.auto_fire = false
	
	Player.health = 6
	Player.max_health = 6
	
	Player.collider = {
		left = 100 - Player.radius,
		right = 100 + Player.radius,
		top = 100 - Player.radius,
		bottom = 100 + Player.radius
	}
end

function update_player_position()
	movement_vector = {
		x = ((btn(1) or key("d")) and 1 or 0) - ((btn(0) or key("a")) and 1 or 0),
		y = ((btn(3) or key("s")) and 1 or 0) - ((btn(2) or key("w")) and 1 or 0)
	}
	if (movement_vector.x == 0 and movement_vector.y == 0) return
	
	magnitude = sqrt(movement_vector.x^2 + movement_vector.y^2)
	movement_vector.x /= magnitude
	movement_vector.y /= magnitude
	x += movement_vector.x*Player.speed
	y += movement_vector.y*Player.speed
	
	if (x - Player.radius < 0) then
		x = Player.radius
	end
	if (x + Player.radius > 480) then
		x = 480 - Player.radius
	end
	if (y + Player.radius > 260) then
		y = 260 - Player.radius
	end
	if (y - Player.radius < 0) then
		y = Player.radius
	end
	
	Player.collider = {
		left = x - Player.radius,
		right = x + Player.radius,
		top = y - Player.radius,
		bottom = y + Player.radius
	}
end

function update_player_attack()
	if ( btn(4) or keyp("q") ) Player.auto_fire = not Player.auto_fire
	
	if ( (btn(5) or key("space") or Player.auto_fire) and ( (t() - Player.attack_last_time) >= Player.attack_speed) ) then
		Player.attack_last_time = t()
		for i = 1, #Player.attack_sides do
			shoot_side(Player.attack_sides[i])
		end
		
		if (Player.attacks_per_fire >= Player.sides) then
			Player.attack_sides = {}
			for i = 1, Player.sides do
				add(Player.attack_sides, i)
			end
		else
			local current_last_side = Player.attack_sides[1]
			Player.attack_sides = {}
			for i = current_last_side+1, Player.attacks_per_fire + current_last_side do
				add(Player.attack_sides, i)
			end
			
			for i = 1, #Player.attack_sides do
				if (Player.attack_sides[i] > Player.sides) Player.attack_sides[i] -= Player.sides
			end
		end
	end
end

function update_player_angle()
	for i = #enemies, 1, -1 do
		if (check_bounded_collision(Player.collider, enemies[i].collider) and enemies[i].spawned) then
			sfx(2)
			Player.health -= 1
			deli(enemies, i)
		end
	end
	
	-- Subtract from y, because (0, 0) is located at top-left.
	local y_difference = y - mouse_y
	local x_difference = mouse_x - x
	-- Add 90 to offset the angle correctly from rightmost position of circle; (1, 0)
	local angle = atan2(y_difference, x_difference)*360+90
	
	local angle_increment = 360 / Player.sides
	
	for side_number = 1, Player.sides, 1 do
		Player.points[side_number] = {
			x = x + flr(cos(degree*(angle_increment*(side_number-1)+angle))*Player.radius),
			y = y + flr(sin(degree*(angle_increment*(side_number-1)+angle))*Player.radius)
		}
	end
end

function draw_player()
	draw_bounded_collider(Player.collider)

	if (Player.auto_fire) print("Autofire Enabled")
	color(12)
	for i = 1, #Player.points do
		local used_color = 16
		if (contains(Player.attack_sides, i)) used_color = 28
		
		if (i == #Player.points) then
			line(Player.points[i].x, Player.points[i].y, Player.points[1].x, Player.points[1].y, used_color)
	   else
	   	line(Player.points[i].x, Player.points[i].y, Player.points[i+1].x, Player.points[i+1].y, used_color)
	  	end
	end
	
	draw_bar(x+1, y-8, Player.radius*2+1, 4, Player.health, Player.max_health, "blue")
	draw_bar(x+1, y-6, Player.radius*2+1, 3, t() - Player.attack_last_time, Player.attack_speed, "white")
end