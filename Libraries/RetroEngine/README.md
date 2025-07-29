# Retro Engine
A simple Rust Terminal User Interface (TUI) library with an expansive component system and complete control over logic and style.

# Motive
Upon exploring the creation of CLI tools, I decided to create my own library in order to make interactive terminal applications. Addtionally, I've used this project to more deeply understand Rust generics and traits.

# Features
* Full control over component logic and styling. (Components are standard Rust structs which implement a common `Component` trait.)
* Simple event feeder system. (AutoFeeders allow for more complex UI layouts without sacrificing simplicity.)
* Simple function based api. (Allows you to define your own application loop and structure.)
* High level terminal output interface. (Handles ASCII color codes, emojis or multiple codepoint characters, etc.)

# Preview
A Playground Project showcasing the library's usage will be available soon.

# Usage
This project contains a Rust crate which is not published on crates.io. To install, and use in your own Rust projects you will need to add the following line to your `Cargo.toml` file:

```toml
...
[dependencies]
...
retro_engine = { git = "link needs to be tested" }
```

# License
As with all other projects in this playground, the license is CC BY-NC.
