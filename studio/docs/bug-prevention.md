# Bug prevention

Goal: games run on the engine without strange behavior. Most bugs in an engine that imitates another come from small differences in semantics, not from programming mistakes. This list targets those.

## 1. Error semantics like Roblox's
- An error inside a script kills only that script's thread and goes to the Output with a message and stack. The rest of the game keeps running.
- If a script calls several `start()` functions in sequence and one errors, the following ones do not start, exactly as on Roblox. The engine reproduces this; it does not swallow the error or crash the process.
- A Rust panic becomes a catchable Lua error. It never takes down the window.
- Test: a script that fails on purpose in the middle of a sequence; check the Output and which services started.

## 2. Behaviors games commonly depend on
- `WaitForChild` without a timeout waits forever and warns about an "infinite yield" after 5 s.
- Anything in `ReplicatedStorage` must be replicated to the client **before** the client blocks waiting for it (`WaitForChild`).
- `table.freeze`: writing to a frozen table must raise an error, as in Roblox's Luau.
- `require` caches: the same module returns the same table. A circular `require` is an error, not a hang.
- Player attributes replicate to the client on their own.
- Tags (`CollectionService`) work on both sides.
- `Debris:AddItem` and `task.delay` respect time, including with many pending items at once.

## 3. Invariants (checked in debug builds)
- No NaN or infinity in `Position`, `CFrame`, `Size` or velocity.
- Every Instance has a valid parent or nil; parent cycles are impossible.
- A destroyed Instance referenced from Lua raises a clear Lua error, never an invalid access.
- `Destroy` disconnects events and cancels tweens and emitters of the object.
- At the end of each test, the count of Instances and live emitters returns to its initial value (games create many short-lived effects: leaks show up here).

## 4. Predictable simulation in tests
- Fixed time step (1/60 s) and a seeded `math.random`.
- Injectable clock: a test advances time without waiting (`Runtime::tick(dt)`).
- Injected input (a right click at a world point, a key press) instead of depending on a window.

## 5. Rust rules in the game-execution crates
- No `unwrap`, `expect` or panicking indexing in execution paths (clippy `unwrap_used` and `expect_used` as errors in `dom`, `lua`, `rojo-client`).
- `cargo clippy -- -D warnings`, `cargo fmt --check` and `cargo test` in `harness/check.sh`.
- External input (Rojo responses, `.rbxl` files) is never trusted: decoding returns an error, not a panic.

## 6. Tests
- Each expected behavior becomes a test whose name carries its ID.
- Every bug found becomes a regression test **before** it is fixed.
- Differential tests against Lune (`harness/diff_luau.py`) for the file reader, data types and scheduler.
- Fuzzing of the `.rbxl` reader and the msgpack decoder (cargo-fuzz needs nightly: to confirm).
- Hostile inputs on remotes: NaN, huge distances, wrong types, rapid repetition.

## 7. Before saying "done"
- The criterion was actually executed.
- What could not be tested (window, FPS, visuals on other hardware) is listed with the command to run it.
