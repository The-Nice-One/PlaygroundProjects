glow_copper = readpng("./assets/glow/GlowCopper.png")
glow_iron = readpng("./assets/glow/GlowIron.png")
glow_diamond = readpng("./assets/glow/GlowDiamond.png")
glow_gold = readpng("./assets/glow/GlowGold.png")

glow_mappings = Dict(
    glow_copper => [Point(86, 108)],
    glow_iron => [Point(5, 86), Point(62, 112), Point(120, 107), Point(136, 120)],
    glow_diamond => [Point(110, 124), Point(44, 125)],
    glow_gold => [Point(12, 107)],
)
glow = Dict()
function frame_glow(framenumber)
    global glow

    if framenumber % 10 == 0 && glow == Dict()
        if rand() < 0.5
            choice = rand(keys(glow_mappings))
            push!(glow, choice => glow_mappings[choice][rand(1:length(glow_mappings[choice]))])
        end
    elseif framenumber % 10 == 0
        glow = Dict()
    end
    if glow != Dict()
        key = collect(keys(glow))[1]
        placeimage(key, glow[key], 1.0)
    end
end
