# livemod - Runtime modification of program values

Livemod is my attempt to make Unity-style runtime parameter modification possible in Rust.

## Usage

Livemod requires the library, `livemod`, and a locally-installed viewer, such as `livemod-gui`.

`livemod-gui` can be installed through `cargo`:

```
cargo install livemod-gui
```

And can be used from your code by:

```rs
let livemod = LiveModHandle::new_gui();

let tracked_variable = livemod.create_variable("My variable", 0_u32);
```