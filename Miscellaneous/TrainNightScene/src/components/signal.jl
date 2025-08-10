signal = Animation([Frame(readpng("./assets/signal/SignalGreen.png")),
    Frame(readpng("./assets/signal/SignalRed.png")),
    Frame(readpng("./assets/signal/SignalYellow.png"))])

function frame_signal(framenumber)
    placeimage(current(signal), Point(149, 46), 1.0)

    if framenumber == 34 || framenumber == 197
        cycle!(signal)
    end
end
