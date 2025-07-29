module SimpleFrameAnimation
import Base: length
export Cache, Frame, Animation, AnimationCollection, cycle!, current, length

mutable struct Cache
    varients::Dict{Unsigned, Unsigned}

    Cache() = new(Dict())
end

mutable struct Frame{T}
    data::Union{T, Vector{T}}
    id::Unsigned

    Frame(data::Union{T, Vector{T}}) where T = new{T}(data, 1)
end

mutable struct Animation{T}
    frames::Vector{Frame{T}}
    current::Unsigned
    cache::Cache

    Animation(frames::Vector{Frame{T}}) where T = new{T}(frames, 1, Cache())
end

mutable struct AnimationCollection{T, U}
    animations::Dict{U, Animation{T}}
    current::U

    AnimationCollection(animations::Dict{U, Animation{T}}) where {T, U} = new{T, U}(animations, first(keys(animations)))
end

function Base.getindex(animation::Animation{T}) where T
    return animation.frames[animation.current]
end

function Base.getindex(collection::Animation{T}, index::Unsigned) where T
    return collection.frames[index]
end

function Base.setindex!(animation::Animation{T}, frame::Frame{T}) where T
    animation.frames[animation.current] = frame
end

function Base.setindex!(animation::Animation{T}, frame::Frame{T}, index::Unsigned) where T
    animation.frames[index] = frame
end

"""
    length(animation::Animation{T}) where T

Return `length(animation.frames)`.
"""
function length(animation::Animation{T}) where T
    return length(animation.frames)
end

function cycle!(animation::Animation{T}, framenumber::Signed = 1) where T
    animation.current += framenumber

    if animation.current > length(animation.frames)
        animation.current = 1
    end

    if animation.current < 1
        animation.current = 1
    end
end

function cycle!(collection::AnimationCollection{T, U}, framenumber::Signed = 1) where {T, U}
    animation = collection.animations[collection.current]
    update!(animation, framenumber)
end

function current(animation::Animation{T})::T where T
    if animation.current > length(animation.frames)
        animation.current = 1
    end

    frame_data = animation.frames[animation.current].data
    if isa(frame_data, AbstractVector)
        frame = animation.frames[animation.current]
        if !haskey(animation.cache.varients, frame.id)
            animation.cache.varients[frame.id] = rand(1:length(frame_data))
        end
        return frame_data[animation.cache.varients[frame.id]]
    else
        return frame_data
    end
end

function current(sprite::AnimationCollection{T, U})::T where {T, U}
    animation = sprite.animations[sprite.current]
    return get(animation)
end

end # module SimpleFrameAnimation
