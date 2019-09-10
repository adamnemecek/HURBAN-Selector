# HURBAN Selector

[![Build Status](https://dev.azure.com/subdigital/HURBAN-selector/_apis/build/status/sub-digital.HURBAN-Selector?branchName=master)](https://dev.azure.com/subdigital/HURBAN-selector/_build/latest?definitionId=1&branchName=master)

## Prerequisites

- [Rust](https://rustup.rs/)
- Clippy (`rustup component add clippy`)
- Rustfmt (`rustup component add rustfmt`)
- [Dependencies for `shaderc-sys`](https://github.com/google/shaderc-rs#building-from-source)

## Developing

We use standard `cargo` workflows:

- `cargo clippy` to have a nice chat with 📎, the linter,
- `cargo fmt` to format the project,
- `cargo test` to run tests,
- `cargo doc --open` to build and open local documentation for the
  project and all dependencies (optionally pass
  `--document-private-items`),
- `cargo build` to build,
- `cargo run` to run.

A gpu backend is automatically selected, but optionally a non-default
gpu backend can be specified with the `RTY_GPU_BACKEND` environment
variable.

`RTY_GPU_BACKEND=<GPU-BACKEND> cargo run `, where `<GPU-BACKEND>` is one of:

- `vulkan` on Windows, Linux, or (FIXME) macOS with VulkanSDK/MoltenVK,
- `d3d12` on Windows,
- `metal` on macOS.

If working on the renderer (on a platform supporting Vulkan), enabling
the Vulkan validation layers is useful for having additional
validation:

``` shell
VK_LAYER_PATH=/path/to/VulkanSDK/version/Bin \
VK_INSTANCE_LAYERS=VK_LAYER_KHRONOS_VALIDATION \
cargo run
```
