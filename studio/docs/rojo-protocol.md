# Rojo protocol (as used by this engine)

Read on 2026-10-15 from the source of `rojo-rbx/rojo` (commit `7e60406`, server version **7.7.1**, **MPL-2.0**) and `rojo-rbx/rbx-dom` (commit `dbec31d`, `rbx_types` 3.1.0, MIT), then confirmed against a real `rojo serve` 7.7.1 (fixtures in `studio/harness/fixtures/rojo/`).
Legend: **[code]** read from source · **[confirmed]** observed against a live server · **[to confirm]**.

## Summary
- Protocol version: **5** (`PROTOCOL_VERSION`, `src/web/interface.rs`). The official client refuses a server with a different version; so do we. [code]
- Default port: **34872** (`DEFAULT_PORT`, `src/cli/serve.rs`). The server listens on loopback by default. [code]
- Transport: **MessagePack** for all responses and POST bodies (`Content-Type: application/msgpack`). Structs are encoded as **maps** with text keys, in human-readable mode. [code]
- Note: the header comment of `api.rs` says responses are "generally JSON", but the code uses msgpack. The code wins.
- Errors: msgpack body `{kind, details}` with `kind` in `NotFound | BadRequest | Forbidden | InternalError` (HTTP 404, 400, 403, 500). [code]
- The server validates `Host`/`Origin` (anti DNS rebinding, `src/web/origin.rs`): talk to `localhost` or `127.0.0.1` and do not send a web `Origin`. [code; confirmed: clients without `Origin` work]

## Routes [code: `src/web/api.rs`]
| Method and path | Purpose | Body |
|---|---|---|
| `GET /api/rojo` | Handshake: version, session, project name, root | `ServerInfoResponse` |
| `GET /api/read/{id,id,...}` | Requested instances and **all their descendants**, plus the message cursor | `ReadResponse` |
| `GET /api/socket/{cursor}` | WebSocket upgrade; stream of patches from the cursor | binary msgpack packets |
| `POST /api/write` | Client sends changes back to disk | `WriteRequest` → `WriteResponse` |
| `POST /api/serialize` | Ask for a binary model (.rbxm) of some IDs | `SerializeRequest` → `SerializeResponse` |
| `POST /api/ref-patch` | Resolve references between instances | `RefPatchRequest` → `RefPatchResponse` |
| `POST /api/open/{id}` | Open the instance's file in the user's editor | `OpenResponse` |

Any other route: 404 `NotFound`. `/api/socket/` without an upgrade: 400. A malformed cursor: 400.
The engine only uses **`/api/rojo`, `/api/read`, `/api/socket`**.

## Types (field names on the wire) [code: `src/web/interface.rs`]
`Ref`: in human-readable mode (what Rojo uses) a **32-digit hexadecimal string** (`{:032x}`). `SessionId`: UUID v4 as text.

**ServerInfoResponse** (`GET /api/rojo`), camelCase keys: `sessionId`, `serverVersion`, `protocolVersion` (u64), `projectName`, `expectedPlaceIds`, `unexpectedPlaceIds`, `gameId`, `placeId`, `rootInstanceId` (Ref).

**ReadResponse**: `sessionId`, `messageCursor` (u32), `instances` (map Ref → Instance).

**Instance**, PascalCase keys: `Id`, `Parent`, `Name`, `ClassName`, `Properties` (name → Variant), `Children` (list of Ref), `Metadata` (`{ignoreUnknownInstances: bool}` or null). `SharedString` properties are **filtered out** by the server. [code]

**Variant** (property value): the `rbx_types` enum with serde **externally tagged**: `{"Vector2":[5.0,7.0]}`, i.e. a one-entry map `TypeName → value`.

**InstanceUpdate**: `id`, `changedName`, `changedClassName`, `changedProperties` (name → Variant **or null**, where null means "property removed"), `changedMetadata`.

**SubscribeMessage** (one patch): `removed` (list of Ref), `added` (map Ref → Instance), `updated` (list of InstanceUpdate).

**SocketPacket** (each WebSocket message, **binary frame**): `sessionId`, `packetType` (currently only `"messages"`), `body` = `{messageCursor: u32, messages: [SubscribeMessage]}`.

## Client flow
1. `GET /api/rojo` → decode `ServerInfoResponse`.
2. If `protocolVersion != 5`: **fail with a clear error** (server version, expected version). Do not try to continue.
3. `GET /api/read/{rootInstanceId}` → full tree + `messageCursor` + `sessionId`. Build the `WeakDom` from `instances`; the root's `Parent` is the zero `Ref`. [confirmed]
4. Open a WebSocket at `/api/socket/{messageCursor}`. For each binary frame: decode `SocketPacket`, **check `sessionId`** (if it changed the server restarted: drop the tree and start over), apply each `SubscribeMessage` in order, keep the new cursor.
5. If the WebSocket drops: reconnect with the last cursor, or start over from step 1 if the server answers with an error or a different session.
6. Ping/pong/close follow standard WebSocket behavior; the server ignores messages the client sends. [code]

## Applying a patch to the DataModel
- `removed`: destroy each Ref (and its descendants).
- `added`: insert each Instance **under the Parent it names**; the map order is not guaranteed, so create parents before children. The server already sends the descendants of an added instance in the same `added`.
- `updated`: apply name, class and properties. A null property value = remove it (back to the class default).
- Inconsistent patches (unknown instance, duplicate, orphan, removing the root) are **errors, never panics**; the caller re-reads the whole tree, which is always safe.

## Risks and open points
- **License:** Rojo is MPL-2.0. The engine **reimplements the protocol with its own definitions** and uses MIT libraries for the types, which keeps MPL code out of this repository.
- **Protocol changes:** tests pin Rojo 7.7.1 / protocol 5. A protocol 6 must fail with a clear message.
- **Confirmed with a live `rojo serve` 7.7.1:** `SessionId` as UUID text; `Ref` as 32-digit hex; root `Parent` = zero `Ref`; tagged `Variant`; `Metadata.ignoreUnknownInstances`; `Content-Type: application/msgpack`; each WebSocket frame carried one message and the cursor advanced one at a time (0 → 1 → 2 → 3). Captured patches: a `Source` update, a new file (ModuleScript) and a deleted file.
- **Measured:** a file saved on disk reaches the Rust DataModel in about 66 ms (`crates/rojo-client/tests/live_server.rs`).
- **Still to confirm:** reconnecting after a dropped socket; `Variant` types other than strings in patches (Vector3, CFrame, Color3, Enum).
