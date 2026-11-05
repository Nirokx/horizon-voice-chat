# Horizon Voice Chat

A native Horizon mod (Minecraft 26.3, native hook API 409 and later; tested on Horizon Nightly
2026-11-05, `f319701`) that is a client for
the [Simple Voice Chat](https://github.com/henkelmax/simple-voice-chat) protocol. It uses the same
protocol as the Java mod, so on any server with the voice chat mod or plugin you can talk with players
who play on the regular Java launcher with Simple Voice Chat.

- **Menu (V)**: everything without chat commands. The left side has the connection's status, buttons
  for the microphone, voice chat on/off, push to talk or voice activation and the group names, `-`/`+`
  for voice volume, microphone gain and the activation level (with a live microphone meter), and
  **Hotkeys**: every key of the mod as it is bound now. The right side lists the server's groups
  with their members: **Join** (a password group asks for its password), **Leave** for yours, and
  **New group** (name, optional password, type, Create). Tab moves between fields, Enter submits,
  Escape or V closes, the wheel scrolls the groups.
- **Talking**: hold **Caps Lock** (push to talk), or switch to voice activation. Voices come from where
  the players stand: they get quieter with distance and pan left or right, as the Java mod's do.
- **Keys** (Options > Mods > Horizon Voice Chat): the menu (V), push to talk (Caps Lock), whisper (not
  bound), mute the microphone (J), voice chat off/on (K). The menu shows whatever you bind them to.
- **Groups**: each player's group shows above their head (`[Builders]`; yours in green, others in
  yellow). Your group and its members are on screen, and `/vc groups` lists every group with its members.
  Join, leave and create them in the menu, or with `/vc join <group> [password]`, leave with `/vc leave`, make one with
  `/vc create <name> [password] [normal|open|isolated]`. The server's own `/voicechat` command works too.
- **On screen** (top left): connecting, talking or whispering, muted, voice chat off, your group, and
  who is talking.
- **`/vc`**: `status`, `mute`/`unmute`, `off`/`on`, `mode ptt|voice`, `volume <0-200>`,
  `volume <player> <0-400>`, `gain <10-500>`, `threshold <dB>`, and the group commands above.
- **Settings page**: activation, microphone, voice chat, voice volume, microphone gain, voice activation
  level, protocol, status on screen, group names above players.

Both Simple Voice Chat protocols are supported: **2.6.x** (compatibility version 20, the current
releases) and **2.7** (21, the pre-release). The mod asks with 2.6.x first and then 2.7, and remembers
which one each server answered. On a 2.7 server the first visit may show "Your voice chat client version
is not compatible" once, before the 2.7 request succeeds. Settings > Protocol fixes the version if you
prefer.

It needs **libopus** and uses the default microphone and speakers. It looks for libopus in its own data
folder first (`horizon-mods/.horizon-mod-data/voicechat/`: `opus.dll` on Windows, `libopus.0.dylib`
on macOS, `libopus.so.0` on Linux), then the system's (the `opus` package; already installed on most
Linux desktops). Windows has no system libopus: build `opus.dll` with `tools/build_opus.py` (below) and
put it in that folder.

## Not affiliated

This is an unofficial project. It is not made, endorsed or supported by Max Henkel or the Simple
Voice Chat project; please don't report its problems to them. "Simple Voice Chat" is named to say
which mod and plugin it works with.

## Where the protocol code comes from

This is **not** a clean-room implementation. The mod was written with an AI coding assistant
(Claude Code), which read [Simple Voice Chat's source](https://github.com/henkelmax/simple-voice-chat)
on GitHub (its packet classes, `AudioChannel`, `MicThread` and some config defaults) to implement the
protocol. No code is copied verbatim (this is Rust, Simple Voice Chat is Java), but the protocol and
audio code follow Simple Voice Chat's implementation, and comments name its classes. Simple Voice Chat
is "Copyright (c) Max Henkel - All Rights Reserved". If you are its author and want this changed or
taken down, open an issue and it will be.

## Build and install

Uses the Horizon launcher's own Rust and Horizon source (nothing else to install):

```sh
L=~/.local/share/horizon-launcher
export PATH=$L/rust/1.99.1-x86_64-unknown-linux-gnu/bin:$PATH CARGO_HOME=$L/cargo
(cd mixins/voicechat/engine && cargo test --offline && rm -f Cargo.lock)   # the engine's tests
python3 tools/package.py --install        # target/voicechat-1.1.0.hnmod, copied into the launcher's native mods
```

Then build the game with its mods in the launcher. The mod declares `"hooks": 409`: the server's
payloads come through the `client_packet` hook point (API 408) and go out through `send_packet`
(API 409), so the launcher builds it on any Horizon with that API. Its one reach into the game's own
state is `mixins/voicechat/world.rs`, a module of `game::remote` that reads the joined address, the
other players and the camera (the hook points don't offer those to a client yet); a Nightly that
moves those fields breaks the build there and nowhere else.

On Windows, `opus.dll` from the official libopus source with the launcher's own MinGW:

```sh
# opus-x.y.z from https://downloads.xiph.org/releases/opus/ (check SHA256SUMS.txt)
python tools/build_opus.py <opus-x.y.z dir> "%APPDATA%/horizon-launcher/rust/<ver>-x86_64-pc-windows-gnu/mingw64/bin/gcc.exe" out
# copy out/opus.dll (and out/COPYING.libopus) into horizon-mods/.horizon-mod-data/voicechat/
```

For tests without a microphone, `HORIZON_VOICECHAT_TEST_TONE=1` sends a steady 440 Hz tone instead of
the microphone, and the log (`logs/latest.log`, lines `Voice Chat: …`) counts the packets sent and heard.

## Layout

- `mixins/voicechat/engine/`: the mod's own crate (built once and cached), with its unit tests
  - `proto.rs`: Simple Voice Chat's packets, both versions; `buf.rs`: `FriendlyByteBuf`
  - `crypt.rs`: AES-128-GCM, as `Secret` encrypts; `opus.rs`: libopus, loaded at run time
  - `net.rs`: the UDP connection (authenticate, check, keep alive) and the mic thread
  - `mix.rs`: jitter buffers, distance and panning, resampling; `audio.rs`: microphone and speakers (cpal)
  - `ui.rs`: the menu; `labels.rs`: group names above players; `lib.rs`: the session, keys, HUD, `/vc` and settings
- `mixins/voicechat/*.rs`: one small hook adapter per hook point
- `mixins/voicechat/horizon-mixin.toml`: where each hook goes
- `tools/package.py`: makes the `.hnmod`

## Tested

- Engine unit tests: the packet formats of both versions, the AES-GCM test vector, Opus through the
  system libopus, the mixer, distance and panning, the session, the group display, and the menu
  (hotkeys and groups shown, joining a password group, typing a new group).
- The bundle applies to Horizon `c9d474d` with horizon-mixins 0.6.0, and the modded client
  type-checks (`cargo check -p horizon-client`).
- Live, against Paper 26.3 with the Simple Voice Chat 2.6.27 plugin: two clients driven by this engine
  authenticated and connected to voice chat; one heard the other's voice (a test tone through
  Opus and the server); creating and joining groups, the member lists and the labels above players all
  came from the server.
- 1.1.0 on Horizon `f319701` (and `main` at `63eccfc`): the bundle applies and the game builds; live
  against Paper **1.21.11** (through Horizon's older-servers adapter) with the Simple Voice Chat
  2.6.27 plugin, two Horizon clients connected to voice chat and one heard 1000+ sound packets of the
  other's test tone, on Windows with the built `opus.dll`.
- Not tested yet: the menu on screen in the real game, 2.7 servers live (no 2.7 plugin build is published), a real microphone and speakers
  in the game, and talking with the Java mod itself.

## License

MIT, see [LICENSE](LICENSE), for the parts that are this project's own work. Simple Voice Chat itself is
not covered by this license and remains its author's, including whatever of its design the protocol
code follows (see above).
