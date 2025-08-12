--[[pod_format="raw",created="2025-08-02 13:14:24",modified="2025-08-02 21:10:23",revision=157]]
function init_ui()
	Ui = {
		tutorial_stage = 0,
		buttons = {},
		shop_open = false,
		upgrades = {
			{name = "Player Sides", level=1, base_cost=5, col=12},
			{name = "Player Health", level=1, base_cost=5, col=12},
			{name = "Player Speed", level=1, base_cost=5, col=12},
			
			{name = "Projectile Damage", level=1, base_cost=5, col=8},
			{name = "Projectile Pierce", level=1, base_cost=5, col=8},
			{name = "Projectile Speed", level=1, base_cost=5, col=8},
		
			{name = "Attack Reload", level=1, base_cost=5, col=11},
			{name = "Attack Sides", level=1, base_cost=5, col=11},
			
			{name = "Radius", level=1, base_cost=5, col=31},
		}
	}
	mouse_collider = {
		left = mouse_x-1,
		right = mouse_x + 4,
		top = mouse_y-1,
		bottom = mouse_y + 2
	}
	new_button(474, 261, 3, 8, "^", 7, "shop")
	local current_y = 158
	for i = 1, #Ui.upgrades do
		current_y += 10
		new_button(473, current_y, 5, 8, "+", 7, Ui.upgrades[i].name)
	end
end

function new_button(x, y, width, height, text, col, id)
	add(Ui.buttons, {
		x = x,
		y = y,
		width = width,
		height = height,
		text = text,
		col = col,
		collider = {
			left = x,
			right = x + width,
			top = y,
			bottom = y + height
		},
		id = id
	})
end

function upgrade(id, index)
	local button = Ui.buttons[index]
	local upgrade = Ui.upgrades[index-1]
	if (upgrade.level == 6) return
	local upgrade_cost = calculate_cost(upgrade.base_cost, upgrade.level)
	print(id, 100, 100)
	if (id == "Player Sides" and coins >= upgrade_cost) then
		Player.sides += 1
		upgrade.level += 1
		coins -= upgrade_cost
	end
	if (id == "Player Health" and coins >= upgrade_cost) then
		Player.max_health += 1
		Player.health = Player.max_health
		upgrade.level += 1
		coins -= upgrade_cost
	end
	if (id == "Player Speed" and coins >= upgrade_cost) then
		Player.speed += 0.3
		upgrade.level += 1
		coins -= upgrade_cost
	end
	
	if (id == "Projectile Damage" and coins >= upgrade_cost) then
		Player.projectile_damage += 1
		upgrade.level += 1
		coins -= upgrade_cost
	end
	if (id == "Projectile Pierce" and coins >= upgrade_cost) then
		Player.projectile_pierce += 1
		upgrade.level += 1
		coins -= upgrade_cost
	end
	if (id == "Projectile Speed" and coins >= upgrade_cost) then
		Player.projectile_speed += 1
		upgrade.level += 1
		coins -= upgrade_cost
	end
	
	if (id == "Attack Reload" and coins >= upgrade_cost) then
		Player.attack_speed -= 0.15
		upgrade.level += 1
		coins -= upgrade_cost
	end
	if (id == "Attack Sides" and coins >= upgrade_cost) then
		Player.attacks_per_fire += 1
		upgrade.level += 1
		coins -= upgrade_cost
	end
	if (id == "Radius" and coins >= upgrade_cost) then
		Player.radius += 1
		upgrade.level += 1
		coins -= upgrade_cost
	end
end

function update_buttons()
	for i = 1, #Ui.buttons do
		if check_bounded_collision(mouse_collider, Ui.buttons[i].collider) then
			Ui.buttons[i].col = 6
			if (Ui.buttons[i].id == "shop" and mouse_p) then
				Ui.shop_open = not Ui.shop_open
				if (Ui.tutorial_stage == 3) then
					Ui.tutorial_stage = 4
				end
				if (Ui.tutorial_stage == 2) then
					Ui.tutorial_stage = 3
				end
			elseif (mouse_p) then
				upgrade(Ui.buttons[i].id, i)
			end
		else
			Ui.buttons[i].col = 7
			if (i > 1 and Ui.upgrades[i-1].level == 6) Ui.buttons[i].col = 6
		end
	end
end

function update_ui()
	mouse_collider = {
		left = mouse_x-1,
		right = mouse_x + 4,
		top = mouse_y-1,
		bottom = mouse_y + 2
	}
	
	if (Ui.tutorial_stage == 0 and (x != 100 or y != 100)) then
		Ui.tutorial_stage = 1
	end
	if (Ui.tutorial_stage == 1 and (btn(5) or key("space"))) then
		Ui.tutorial_stage = 2
	end
	
	update_buttons()
end

function calculate_cost(base_cost, level)
	return flr(base_cost * level^2)
end

function draw_upgrade_board()
	local current_x = 298
	local current_y = 160
	print("-- Upgrades Shop --", current_x, current_y, 29)
	for i = 1, #Ui.upgrades do
		current_y += 10
		
		local upgrade = Ui.upgrades[i]
		local color_set = nil
		if (upgrade.col == 12) color_set = "blue"
		if (upgrade.col == 8) color_set = "red"
		if (upgrade.col == 11) color_set = "green"
		if (upgrade.col == 31) color_set = "orange"
		
		print(upgrade.name, current_x, current_y, upgrade.col)
		draw_bar(current_x+120, current_y+12, 60, 8, upgrade.level, 6, color_set)
		if (upgrade.level == 6) then print("-000", current_x+154, current_y, 10)
		else print("-" .. pad_number(calculate_cost(upgrade.base_cost, upgrade.level), 3), current_x+154, current_y, 10) end
	end
end

function draw_buttons()
	for i = 1, #Ui.buttons do
		local button = Ui.buttons[i]
		if (i == 1 or Ui.shop_open) then
			draw_bounded_collider(button.collider)
			print(button.text, button.x, button.y, button.col)
		end
	end
end

function draw_ui()
	draw_bounded_collider(mouse_collider)
	
	local coins_string = "Coins: " .. pad_number(coins, 6)
	
	rectfill(0, 260, 480, 270, 0)
	print(get_tutorial_message(), 1, 261, 16)
	print(coins_string, 410, 261, 10)
	draw_buttons()
	
	if (Ui.shop_open) draw_upgrade_board()
end

function get_tutorial_message()
	if (Ui.tutorial_stage == 0) then
		return "Tutorial: Move with W,A,S,D keys or Arrow keys."
	end
	if (Ui.tutorial_stage == 1) then
		return "Tutorial: Fire projectile(s) on the glowing side(s) with Space Key."
	end
	if (Ui.tutorial_stage == 2) then
		return "Tutorial: Open the shop with the ^ button on the right to purchase upgrades."
	end
	if (Ui.tutorial_stage == 3) then
		return "Tutorial: Close the shop with the same ^ button on the right."
	end
	if (Ui.tutorial_stage == 4) return "Game: Good Luck! Enemies will spawn now..."
	if (Ui.tutorial_stage == 5) return "Game: Good Game! Your score was " .. score
	return ""
end