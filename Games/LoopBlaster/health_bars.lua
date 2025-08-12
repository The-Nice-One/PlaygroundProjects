--[[pod_format="raw",created="2025-08-01 01:15:24",modified="2025-08-02 19:14:07",revision=43]]
function draw_bar(x, y, width, height, health, max_health, color_set)
	if (health > max_health) health = max_health
	percentage_health = (100 / max_health) * health
	filled_pixels = (width / max_health) * health
	
	local used_color = nil
	if (color_set == "blue") then
		used_color = 28
		if (percentage_health < 68) used_color = 12
		if (percentage_health < 34) used_color = 16
	elseif (color_set == "red") then
		used_color = 8
		if (percentage_health < 68) used_color = 24
		if (percentage_health < 34) used_color = 2
	elseif (color_set == "white") then
		used_color = 7
		if (percentage_health < 68) used_color = 6
		if (percentage_health < 34) used_color = 5
	elseif (color_set == "green") then
		used_color = 11
		if (percentage_health < 68) used_color = 27
		if (percentage_health < 34) used_color = 3
	elseif (color_set == "orange") then
		used_color = 31
		if (percentage_health < 68) used_color = 4
		if (percentage_health < 34) used_color = 20
	end
	

	rrectfill(x-width/2, y-height-5, filled_pixels, height, 2, used_color)
	rrect(x-width/2, y-height-5, width, height, 2, 0)
end