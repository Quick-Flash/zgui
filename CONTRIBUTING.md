# Contributing to zgui

Thanks for your interest in zgui. Questions and ideas are welcome on
[Discord](https://discord.gg/E6SDmyRRmd) and in GitHub issues.

## Setup

- The toolchain is the nightly pinned in `rust-toolchain.toml`. rustup installs it on first use.
  There is no stable-Rust build.
- On Linux, install the windowing and font libraries: `pkg-config`, `libxkbcommon`, `libwayland`,
  the X11 libraries (`libx11`, `libxcb`, `libxcursor`, `libxi`, `libxrandr`) and `fontconfig`.
- With Nix, `nix develop` gives you all of the above.

## Before you open a pull request

Run the full local gate:

```console
cargo xtask ci
```

It runs formatting, clippy, the build, all tests and doc tests, the performance and invalidation
gates, rustdoc with warnings denied, and the repository checks (`cargo xtask ledger`). GitHub runs
`cargo xtask ci --hosted`, which skips the gates that need a fixed reference machine.

Each gate is also a subcommand, so you can run only the one that covers your change
(`cargo xtask budget`, `cargo xtask skips`, `cargo xtask ledger <name>`, and so on).

CI also runs some checks that are too slow for the local gate. Run them when you change the code
they cover:

```console
cargo miri test -p zgui-arena                     # arena aliasing
cargo miri test -p zgui-dom --test cells          # document aliasing (see ci.yml for the flags)
cargo test -p zgui-bits --features loom           # DirtyCell interleavings
cargo test -p zgui-profile --release --features counters
cargo xtask tsan                                  # threaded targets under ThreadSanitizer
```

## Rules

- **Keep the tree green.** Do not use `#[ignore]` to make a test pass. A test that cannot run on
  every machine checks for what it needs, prints why it skipped on standard error, and returns.
- **Budgets go down freely and up only with a reason.** The invalidation tests assert exact
  amounts of work. To raise one, use a separate commit that changes only that number and gives the
  reason in the test's doc comment.
- **New gates prove they fail.** A new gate or ledger rule comes with a fixture or mutation that
  makes it fail. Ledger rules keep theirs under `xtask/fixtures/<check>/`.
- **Dependencies live in one place.** Declare them in the root `[workspace.dependencies]` and
  inherit them with `foo.workspace = true`.
- **New crates use the generator.** `cargo xtask new-crate zgui-name --layer L4` writes a manifest
  and a crate root with the workspace lints.

## Adapted code

You can adapt code from projects with a compatible license. Start the file with a header that
names the source, and add a row to `NOTICE` under **Derived files**:

```rust
// DERIVED-FROM: the GPUI project, crates/gpui_wgpu/src/shaders.wgsl (Apache-2.0)
// Portions of this file are adapted from that work, licensed under the
// Apache License, Version 2.0, and have been modified for this renderer.
```

## Style

- Prefer small, focused modules. Split a file into submodules when it needs section comments.
- Every public item has rustdoc. Write it for a reader who has only the published documentation.
- Keep commit messages short and about what changed.

## License

By contributing, you agree that your contributions are licensed under [Apache-2.0](LICENSE).
