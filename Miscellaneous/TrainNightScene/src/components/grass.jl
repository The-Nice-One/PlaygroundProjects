grass = Animation([Frame(readpng("./assets/grass/Grass0.png")),
    Frame(readpng("./assets/grass/Grass1.png")),
    Frame(readpng("./assets/grass/Grass2.png")),
    Frame(readpng("./assets/grass/Grass3.png"))])

grass_cycle = false
function frame_grass(framenumber)
    global grass_cycle

    placeimage(current(grass), Point(1, 21), 1.0)

    if rand() < 0.01 && !grass_cycle
        grass_cycle = true
    end

    if grass_cycle && framenumber % 3 == 0
        cycle!(grass)
        if grass.current == 1
            grass_cycle = false
        end
    end
end
