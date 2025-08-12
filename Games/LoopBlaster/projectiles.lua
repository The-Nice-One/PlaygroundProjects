--[[pod_format="raw",created="2025-07-31 21:21:19",modified="2025-08-11 18:06:21",revision=80]]
function init_projectiles()
	projectiles = {}
end

function shoot_side(side_index)
	sfx(0)
	
	local point1_index = side_index
	local point2_index = side_index + 1
	if (side_index == Player.sides) point2_index = 1
	
	local point1_x = Player.points[point1_index].x
	local point2_x = Player.points[point2_index].x
	local point1_y = Player.points[point1_index].y
	local point2_y = Player.points[point2_index].y
	
	local slope_x, slope_y = 0, 0
	-- Correctly calculates the slope
	if point2_x >= point1_x then
		slope_x, slope_y = point2_x - point1_x, point1_y - point2_y
	else
		slope_x, slope_y = point1_x - point2_x, point1_y - point2_y
	end
	
	-- Negative reciprocal for y is positive 1 because of Picotron's coordinate system
	local neg_x, neg_y = -1, 1
	-- Only calculate the negative reciprocal if the slope is not 0
	if (slope_x != 0) then
		neg_x = -(1.0 / slope_x)
	end
	if (slope_y != 0) then
		neg_y =  -(1.0 / slope_y)
	end
	
	-- Some patches in order to ensure the projectiles are moveing the correct direction
	if (point1_y > point2_y) then
		neg_y *= -1
		neg_x *= -1
	end
	if (point2_x < point1_x) then
		neg_y *= -1
	end
	
	local magnitude = sqrt(neg_x^2 + neg_y^2)
	
	local point1_collider_x = point1_x
	local point1_collider_y = point1_y
	local point2_collider_x = point2_x
	local point2_collider_y = point2_y
	
	if (point1_x > point2_x) then
		point1_collider_x = point2_x
		point2_collider_x = point1_x
	end
	
	if (point2_y > point1_y) then
		point1_collider_y = point2_y
		point2_collider_y = point1_y
	end
	
	projectiles[#projectiles+1] = {
		point1 = {
			x = point1_x,
			y = point1_y
		},
		point2 = {
			x = point2_x,
			y = point2_y
		},
		x_change = neg_x / magnitude,
		y_change = neg_y / magnitude,
		collider = {
			left = point1_collider_x,
			right = point2_collider_x,
			top = point2_collider_y,
			bottom = point1_collider_y
		},
		pierce = Player.projectile_pierce,
		enemies_attacked = {},
		damage = Player.projectile_damage
	}
end

function update_projectiles()
	for i = #projectiles, 1, -1 do
		calculated_x_change = projectiles[i].x_change * Player.projectile_speed
		calculated_y_change = projectiles[i].y_change * Player.projectile_speed
		
		projectiles[i].point1.x += calculated_x_change
		projectiles[i].point2.x += calculated_x_change
		
		projectiles[i].point1.y += calculated_y_change
		projectiles[i].point2.y += calculated_y_change
		
		projectiles[i].collider.left += calculated_x_change
		projectiles[i].collider.right += calculated_x_change
		projectiles[i].collider.top += calculated_y_change
		projectiles[i].collider.bottom += calculated_y_change
		
		if (projectiles[i].collider.bottom < 0 or
			projectiles[i].collider.top > 270 or
			projectiles[i].collider.left < 0 or
			projectiles[i].collider.right > 470) then
			deli(projectiles, i)
		else
			for j = #enemies, 1, -1 do
				if (check_bounded_collision(projectiles[i].collider, enemies[j].collider)
					and enemies[j].spawned
					and not contains(projectiles[i].enemies_attacked, enemies[j].spawn_rnd)) then
					sfx(1)
					add(projectiles[i].enemies_attacked, enemies[j].spawn_rnd)
					damage_enemy(j, projectiles[i].damage)
					projectiles[i].pierce -= 1
					if (projectiles[i].pierce < 1) then
						deli(projectiles, i)
						break
					end
				end
			end
		end
	end
end

function draw_projectiles()
	color(28)
	for i = 1, #projectiles do
		draw_bounded_collider(projectiles[i].collider)
		line(projectiles[i].point1.x, projectiles[i].point1.y, projectiles[i].point2.x, projectiles[i].point2.y, 28)
	end
end