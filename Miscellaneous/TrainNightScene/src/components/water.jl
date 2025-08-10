water_bar = readpng("./assets/water/WaterBar.png")
water = Animation([Frame(readpng("./assets/water/Water0.png")),
    Frame(readpng("./assets/water/Water1.png")),
    Frame(readpng("./assets/water/Water2.png")),
    Frame(readpng("./assets/water/Water3.png")),
    Frame(readpng("./assets/water/Water4.png")),
    Frame(readpng("./assets/water/Water5.png")),
    Frame(readpng("./assets/water/Water6.png")),
    Frame(readpng("./assets/water/Water7.png")),
    Frame(readpng("./assets/water/Water8.png")),
    Frame(readpng("./assets/water/Water9.png")),
    Frame(readpng("./assets/water/Water10.png")),
    Frame(readpng("./assets/water/Water11.png"))])

water_bars = []
water_x = 246
run_water = true
function frame_water(framenumber)
    global water_x, run_water

    if water_x - 5 > 184
        for i in 1:5
            placeimage(water_bar, Point(water_x - i, 131), 1.0)
        end

        if framenumber % 5 == 0
            water_x -= 1
        end
    else
        if run_water
            placeimage(current(water), Point(179, 131), 1.0)
        end
        if water.current == length(water.frames)
            run_water = false
        end
        if framenumber % 5 == 0 && run_water
            cycle!(water)
        end
    end
end
