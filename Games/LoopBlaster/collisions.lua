--[[pod_format="raw",created="2025-08-01 00:30:47",modified="2025-08-11 18:09:06",revision=12]]
function check_bounded_collision(collider1, collider2)
	return collider1.left < collider2.right and
		collider1.right > collider2.left and
		collider1.top < collider2.bottom and
		collider1.bottom > collider2.top
end

function draw_bounded_collider(collider, collider_color)
	if (not Debug.enabled) return
	local collider_color = collider_color or 10
	rect(collider.left, collider.top, collider.right, collider.bottom, collider_color)
end