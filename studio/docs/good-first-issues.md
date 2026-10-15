# Good first issues

Small, self-contained tasks. Each one should come with a test. Open an issue (or a PR) and mention which one you take.

1. **Derive `Position` from `CFrame`** for `BasePart` (reading `part.Position` after setting only `CFrame` currently errors). Needs a thin layer or a documented fork of `lune-roblox`; see `docs/decisions.md`.
2. **`Orientation` / `Rotation` / `Size` setters** that keep `CFrame` consistent.
3. **`RunService`**: `Heartbeat`, `Stepped`, `RenderStepped` events driven by `Runtime::tick`, with the Roblox ordering documented and tested.
4. **`Debris:AddItem`** on the virtual clock, with a test that many items expire in order.
5. **`CollectionService`**: `AddTag`, `HasTag`, `GetTagged`, and the `GetInstanceAddedSignal` family.
6. **`TweenService`** for `Part` properties (position, size, color, transparency) with the common easing styles.
7. **Render `Part.Material` and `Part.Transparency`** variants (Plastic, Neon, SmoothPlastic) in `crates/app`.
8. **Textured ground** from `Part.Material` without any Roblox texture: procedural patterns only.
9. **Rapier colliders** for anchored Parts and a kinematic character controller.
10. **Reconnect** in `rojo-client::Session` after a dropped WebSocket (resume from the last cursor), with a test that kills and restarts `rojo serve`.
11. **A blocky character rig built from primitives** (head, torso, arms, legs) with a simple walk animation.
12. **Docs:** a short guide on writing a minimal game for the engine using `sample-game` as a base.
