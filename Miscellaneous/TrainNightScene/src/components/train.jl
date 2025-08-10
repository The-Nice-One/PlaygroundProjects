# Assets
static_railcar = readpng("./assets/static/StaticRailCar.png")
static_window_filled = readpng("./assets/static/StaticWindowFilled.png")

wheel = Animation([Frame(readpng("./assets/wheel/Wheel0.png")),
    Frame(readpng("./assets/wheel/Wheel1.png")),
    Frame(readpng("./assets/wheel/Wheel2.png")),
    Frame(readpng("./assets/wheel/Wheel3.png"))])

chain = Animation([Frame(readpng("./assets/chain/Chain0.png")),
    Frame(readpng("./assets/chain/Chain1.png"))])

# Structs
struct RailCar
    windows::Vector{Bool}
    RailCar() = new([])
end

# Define passengers in each car.
railcars = []
for i in 1:4
    car = RailCar()
    for i in 1:8
        if rand() > 0.6
            push!(car.windows, true)
        else
            push!(car.windows, false)
        end
    end
    push!(railcars, car)
end

function render_car(x, railcar::RailCar)
    offset_y = 39
    placeimage(static_railcar, Point(x, offset_y), 1.0)

    # Wheels
    placeimage(current(wheel), Point(x + 7, offset_y + 23), 1.0)
    placeimage(current(wheel), Point(x + 14, offset_y + 23), 1.0)
    placeimage(current(wheel), Point(x + 68, offset_y + 23), 1.0)
    placeimage(current(wheel), Point(x + 73, offset_y + 23), 1.0)

    # Filled Windows
    window_x_offsets = [3, 23, 30, 37, 44, 51, 58, 78]
    for i in 1:8
        if railcar.windows[i]
            placeimage(static_window_filled, Point(x + window_x_offsets[i], offset_y + 9), 1.0)
        end
    end
end

train_x = -static_railcar.width
function frame_train(framenumber)
    global train_x

    offset_x = 0
    for i in eachindex(railcars)
        render_car(train_x + offset_x, railcars[i])

        # Place Chains
        if i != length(railcars)
            placeimage(current(chain), Point(train_x + offset_x - 3, 39 + 14), 1.0)
            placeimage(current(chain), Point(train_x + offset_x - 3, 39 + 17), 1.0)
        end

        offset_x += -static_railcar.width - 3
    end

    train_x += 5
    cycle!(wheel)
    cycle!(chain)
end
