--[[pod_format="raw",created="2025-08-01 19:11:30",modified="2025-08-02 19:22:27",revision=1]]
function contains(list, value)
	for _, item in pairs(list) do
		if (item == value) return true
	end
	return false
end

function pad_number(number, digits)
	padding = ""
	for i = 1, digits-#("" .. number) do
		padding = padding .. "0"
	end
	return padding .. number
end