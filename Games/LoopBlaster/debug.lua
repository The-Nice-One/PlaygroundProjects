--[[pod_format="raw",created="2025-07-31 21:22:25",modified="2025-08-11 18:07:55",revision=11]]
function init_debug()
	Debug = {}
	Debug.enabled = false
end

function update_debug()
	if (keyp("tab")) Debug.enabled = not Debug.enabled
end

function draw_debug()
	if (Debug.enabled) then
		-- Repositions the print cursor at the top-left corner
		print("\0", 0, 0)
		
		print("Time: " .. flr(t()) .. " seconds", 29)
		print("Mouse: X: " .. mouse_x .. ", Y: " .. mouse_y, 29)
	end
end