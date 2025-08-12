--[[pod_format="raw",created="2025-08-11 18:01:12",modified="2025-08-11 18:10:39",revision=3]]
function init_drops()
	spawned_coins = {}
	spawned_health = {}
end

function spawn_drop(x, y, amount)
	if (rnd(1) < 0.01) then spawn_health(x, y) else spawn_coin(x, y, amount) end
end

function spawn_coin(x, y, amount)
	spawned_coins[#spawned_coins + 1] = {
		x = x,
		y = y,
		amount = amount,
		
		t_offset = t() - flr(t()),
		
		collider = {
			left = x-2,
			right = x+2,
			top = y-2,
			bottom = y+2
		}
	}
end

function spawn_health(x, y)
	spawned_health[#spawned_health + 1] = {
		x = x,
		y = y,
		
		t_offset = t() - flr(t()),
		
		collider = {
			left = x-2,
			right = x+2,
			top = y-2,
			bottom = y+2
		}
	}
end

function update_drops()
	for i = #spawned_coins, 1, -1 do
		if (check_bounded_collision(spawned_coins[i].collider, Player.collider)) then
			coins += spawned_coins[i].amount
			sfx(3)
			deli(spawned_coins, i)
		end
	end
	for i = #spawned_health, 1, -1 do
		if (check_bounded_collision(spawned_health[i].collider, Player.collider)) then
			if (Player.health != Player.max_health) Player.health += 1
			sfx(3)
			deli(spawned_health, i)
		end
	end
end

function draw_drops()
	for i = 1, #spawned_coins do
		local coin = spawned_coins[i]
		local animation_time = t() - flr(t()) + spawned_coins[i].t_offset
		if (animation_time > 1) animation_time -= 1
		if (animation_time < 0.5) then
			ovalfill(coin.x-1, coin.y-2, coin.x+1, coin.y+2, 10)
			oval(coin.x-1, coin.y-2, coin.x+1, coin.y+2, 9)
		else
			circfill(spawned_coins[i].x, spawned_coins[i].y, 2, 10)
			circ(spawned_coins[i].x, spawned_coins[i].y, 2, 9)
		end
	end
	for i = 1, #spawned_health do
		local health = spawned_health[i]
		local animation_time = t() - flr(t()) + health.t_offset
		if (animation_time > 1) animation_time -= 1
		if (animation_time < 0.5) then
			ovalfill(health.x-1, health.y-2, health.x+1, health.y+2, 8)
			oval(health.x-1, health.y-2, health.x+1, health.y+2, 24)
		else
			circfill(health.x, health.y, 2, 8)
			circ(health.x, health.y, 2, 24)
		end
	end
end