star = AnimationCollection(Dict(
    1 => Animation([Frame(readpng("./assets/star/0Star0.png")),
        Frame(readpng("./assets/star/0Star1.png")),
        Frame([readpng("./assets/star/0Star2a.png"), readpng("./assets/star/0Star2b.png"), readpng("./assets/star/0Star2c.png"), readpng("./assets/star/0Star2d.png")])]),
    2 => Animation([Frame(readpng("./assets/star/1Star0.png")),
        Frame([readpng("./assets/star/1Star1a.png"), readpng("./assets/star/1Star1b.png")]),
        Frame(readpng("./assets/star/1Star2.png"))])
))

mutable struct Star
    x::Int
    y::Int
    animation::Animation
    Star(x, y, animation) = new(x, y, animation)
end

stars = []
function frame_stars(framenumber)
    if framenumber % 8 == 0
        for i in reverse(eachindex(stars))
            if stars[i].animation.current == length(stars[i].animation)
                deleteat!(stars, i)
                continue
            end
            cycle!(stars[i].animation)
        end
    end

    if framenumber % 8 == 0 && framenumber < 336
        if rand() < 0.5
            varient = rand(1:2)
            push!(stars, Star(rand(32:216), rand(1:16), deepcopy(star.animations)[varient]))
        end
    end

    for i in eachindex(stars)
        star = stars[i]
        placeimage(current(star.animation), Point(star.x, star.y), 1.0)
    end
end
