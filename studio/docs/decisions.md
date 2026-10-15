# Technical decisions

Checked on 2026-10-15 with `cargo info` (crates.io) and by reading the upstream repositories. Items marked "to confirm" have not been tested in a build yet.

## Versions and licenses
| Piece | Version | License | Notes |
|---|---|---|---|
| Bevy | **0.19.1** (0.20 was still a release candidate when checked) | MIT or Apache-2.0 | Pinned with `=0.19.1` |
| bevy_rapier3d | 0.37.0 | Apache-2.0 | Bevy 0.19 compatibility **to confirm** at dependency resolution |
| rapier3d | 0.36.1 | Apache-2.0 | |
| mlua | 0.12.2 (feature `luau`) | MIT | Luau built from source |
| rbx_dom_weak | 4.2.0 | MIT | |
| rbx_types | 3.1.0 (feature `serde`) | MIT | |
| rbx_binary | 3.0.1 | MIT | .rbxl / .rbxm |
| rbx_xml | 3.0.1 | MIT | .rbxlx / .rbxmx |
| rbx_reflection_database | 3.0.1+roblox-741 | MIT | Class and property reflection |
| rmp-serde | 1.3.1 | MIT | MessagePack (Rojo protocol) |
| tungstenite | 0.30.0 | MIT or Apache-2.0 | WebSocket |
| lune-roblox | 0.4.0 | **MPL-2.0** | Instance / data types on mlua |
| mlua-luau-scheduler | 0.3.0 | **MPL-2.0** | Async Luau scheduler |
| Rojo (server) | 7.7.1, protocol 5 | **MPL-2.0** | Not MIT |
| Lune (runtime binary) | 0.11.0 | MPL-2.0 | Used as a test oracle |

## MPL-2.0 (Rojo, Lune): what it means here
Copyleft **per file**. Using them as a dependency (crates) or running the binary does not require opening the rest of the project. Copying or modifying their files requires keeping those files under MPL-2.0 and publishing them. Rule: **do not copy Rojo code into our files; reimplement the protocol with our own definitions** (the protocol is an interface, and the data types come from `rbx_types`, which is MIT). This is not legal advice: check before relying on it.

## Decisions
1. **Language and rendering:** Rust + Bevy 0.19.x (wgpu, Vulkan). Keep the renderer behind a trait because Bevy's API moves between versions.
2. **DataModel / Luau bridge:** depend on `lune-roblox` for `Instance`, data types and reflection, and on `mlua-luau-scheduler` for the scheduler (see "Luau bridge" below).
3. **Rojo client:** our own, in Rust, with `ureq`, `tungstenite`, `rmp-serde` and `rbx_types`. See [`rojo-protocol.md`](rojo-protocol.md).
4. **Physics:** Rapier. Character as a kinematic body; minimal Humanoid later.
5. **Test oracle:** the Lune binary reads the same .rbxl and runs the same scripts; `harness/diff_luau.py`.

## Luau bridge (`crates/lua`) — 2026-10-15
Depend on `lune-roblox` 0.4.0 and `mlua-luau-scheduler` 0.3.0 (MPL-2.0, as dependencies only). They cover `Instance.new`, `Vector3`, `CFrame`, `Color3`, reflection and events without rewriting them.

Costs and findings from using them outside Lune:
- The DOM lives in a global registry inside `lune-roblox` (and its internals use `expect`). The engine accesses it through the public `Instance` API. A hot reload creates a new DOM; the old one is not freed yet.
- **`Position` and `CFrame` are independent properties** in `lune-roblox`, and reading `part.Position` after only setting `CFrame` fails ("missing default value"). Roblox derives `Position` from `CFrame`. The `InstanceRegistry` extension point cannot override properties that exist in the reflection database, so a real fix needs a fork of that crate (MPL: the changes must be published) or our own `Instance` layer. **Open issue.** The renderer reads `CFrame` and lets `Position` win if both are set.
- `Scheduler` implements `Drop` and removes Lua app data on **every clone**; share it with `Rc`, never `Clone`.
- A Lua thread cannot yield through a Rust function frame: schedule the chunk's Lua function itself as the thread (`Function` → `push_thread_back`), do not call it from a Rust closure.
- `task.wait` uses a virtual clock advanced by the host (`Runtime::tick(dt)`), so tests are deterministic and the game loop controls time.

## Open
- Librebox (open-source attempt at the Roblox API): not evaluated. Check license, maturity, and whether contributing is better than duplicating.
- Exact Bevy 0.19 × bevy_rapier3d 0.37 compatibility.
