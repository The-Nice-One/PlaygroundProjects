# SimpleFrameAnimation.jl
A Julia package for managing animations based on frame data.

# Motive
This project started as an exploration into Julia syntax and language design. In particular the multiple dispatch paradigm is extensively used to create methods which operate on different types- such as the `Animation` and `AnimationCollection` types introduced by this package. Additionally, this package uses interfaces to integrate with the Julia syntax by defining methods such as  `setindex!` and `getindex`. `Animation` and `AnimationCollection` are also generic structs which allow integration for any type of data.

# Features
* `Animation` struct storing a vector of `Frame` structs which can contain any type of data via generics.
* `setindex!`, `getindex()`, and `length()` defined for `Animation` and `AnimationCollection` structs.
* Randomization and caching for frames which have multiple items of data. (`Frames.data` can be `Vector{T}` or `T` via `Union{}`'s.)
* `AnimationCollection` struct which stores multiple `Animation` structs and allows for easy switching between them. The `Animation`'s can be indexed via a `U` generic type.

# Preview
A Playground Project showcasing the usage of this package will be available soon.

# Usage
This package is not published to the Julia General Registry. You must install this package via the git url and set the correct `subdir`:

```julia
using Pkg
Pkg.add(url="link needs to be tested", subdir="link needs to be tested")
```

# License
As with all other projects in this playground, the license is CC BY-NC.
