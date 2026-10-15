# Contributing

Thanks for your interest! This project is early; small, well-tested changes are the easiest to review.

## The clean-room rule (read this first)

This engine must be built **only from public, open sources**.

- **Do not** decompile, disassemble or inspect the Roblox client, and **do not** paste code derived from it.
- **Do not** add models, meshes, textures, animations, sounds or fonts from Roblox, from the Creator Store, or copies of them found elsewhere (including 3D-model sites). If it looks like a Roblox asset, it is not allowed even when the host page shows a permissive license.
- Behavior is learned from the public documentation, from open-source projects (Rojo, rbx-dom, Lune), and from observation of how games behave. Describe where a number or behavior came from; if you could not confirm it, mark it **"to confirm"** instead of guessing.
- Third-party assets are welcome **only** with a clear open license (CC0, CC-BY, MIT, ...). Add author, link and license to `docs/assets-licenses.md` in the same pull request.
- Rojo and Lune are MPL-2.0: using them as a dependency or running the binary is fine. Do **not** copy their files into this repository.

By opening a pull request you confirm that your contribution follows these rules and that you have the right to submit it under the project's license (MIT or Apache-2.0).

## Development

```bash
./setup.sh
cd studio
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test --workspace
```

- The first build of Bevy takes a long time (tens of minutes); later builds are incremental.
- In the game-execution crates (`dom`, `rojo-client`, `lua`) `unwrap`, `expect` and `panic` are denied by clippy. Return errors; a script or a bad network message must never crash the window.
- Every behavior change comes with a test. For bugs, write the failing test first (see [`studio/docs/bug-prevention.md`](studio/docs/bug-prevention.md)).
- Keep commits focused. Describe *why*, not only *what*.

## Where to start

See [`studio/docs/good-first-issues.md`](studio/docs/good-first-issues.md). If you are unsure whether something fits the scope, open an issue first.

## Scope

In scope: a runtime compatible with the subset of the Roblox API that games written as code use, a Rojo-based workflow, Linux first. Out of scope for now: full Roblox parity, a visual editor, networking/multiplayer, and anything that needs proprietary assets.
