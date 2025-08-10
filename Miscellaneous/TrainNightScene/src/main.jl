using Pkg
Pkg.activate("./")
Pkg.instantiate()

using Random
using SimpleFrameAnimation
using Luxor

include("./components/train.jl")
include("./components/signal.jl")
include("./components/bird.jl")
include("./components/stars.jl")
include("./components/water.jl")
include("./components/grass.jl")
include("./components/glow.jl")

movie = Movie(240, 135, "TrainNightScene")
tempdirectory = "./artifacts"

static_background = readpng("./assets/static/StaticBackground.png")
static_overlay = readpng("./assets/static/StaticOverlay.png")

function frame(scene, framenumber)
    background("black")
    origin(O)

    placeimage(static_background, O, 1.0)

    frame_stars(framenumber)
    frame_water(framenumber)
    frame_grass(framenumber)
    frame_glow(framenumber)
    frame_train(framenumber)
    frame_signal(framenumber)
    frame_bird(framenumber)

    placeimage(static_overlay, O, 1.0)
end

animate(movie, [
        Scene(movie, frame, 0:359)
    ],
    creategif=true,
    tempdirectory=tempdirectory
)
