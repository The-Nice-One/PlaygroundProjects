bird_x_position = 241
bird_y_position = 104
bird = AnimationCollection(Dict(
    "fly_left" => Animation([Frame(readpng("./assets/bird/BirdLeftFly0.png")),
                             Frame(readpng("./assets/bird/BirdLeftFly1.png"))]),
    "fly_right" => Animation([Frame(readpng("./assets/bird/BirdRightFly0.png")),
                              Frame(readpng("./assets/bird/BirdRightFly1.png"))]),
    "left" => Animation([Frame(readpng("./assets/bird/BirdLeft.png"))]),
    "right" => Animation([Frame(readpng("./assets/bird/BirdRight.png"))]),
    "crouched" => Animation([Frame(readpng("./assets/bird/BirdCrouched0.png")),
                             Frame(readpng("./assets/bird/BirdCrouched1.png"))])
))

bird_elapsed_frames = 0
function frame_bird(framenumber)
    global bird_x_position, bird_y_position, bird_elapsed_frames, bird

    if framenumber < 89
        if bird_x_position > 161
            bird.current = "fly_left"
            bird_x_position -= 1
            if framenumber % 2 == 0
                cycle!(bird, 1)
            end
            if framenumber % 11 == 0
                bird_y_position += 1
            end
        elseif bird_x_position > 152
            # Bird is Perching
            bird.current = "left"

            bird_x_position -= 1
            bird_y_position += 1
        end
    else
        # Flip bird
        if framenumber == 103
            bird.current = "right"
        end
        # Bird pecking ground
        if framenumber > 105 && framenumber < 240
            bird.current = "crouched"
            if bird.animations[bird.current].current == 1
                bird_elapsed_frames += 1
                if bird_elapsed_frames > 36
                    cycle!(bird, 1)
                    bird_elapsed_frames = 0
                end
            end
            if bird.animations[bird.current].current == 2
                bird_elapsed_frames += 1
                if bird_elapsed_frames > 3
                    cycle!(bird, 1)
                    bird_elapsed_frames = 0
                end
            end
        elseif framenumber > 240 && framenumber < 248
            bird.current = "fly_right"

            bird_x_position += 1
            bird_y_position -= 1
            cycle!(bird, 1)
        elseif framenumber > 248
            # Bird is Flying
            bird.current = "fly_right"
            bird_x_position += 1
            if framenumber % 2 == 0
                cycle!(bird, 1)
            end
            if framenumber % 11 == 0
                bird_y_position -= 1
            end
        end
    end

    placeimage(current(bird), Point(bird_x_position, bird_y_position), 1.0)
end
