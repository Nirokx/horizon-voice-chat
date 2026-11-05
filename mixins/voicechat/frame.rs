// Off a server (the title screen, single player, a lost connection): the
// voice chat hangs up. And the menu closes when it asked to, and forgets a
// close the game did (Escape, another screen).
fn voicechat_frame(game: &mut crate::game::Game<'_>) {
    use voicechat_engine as vc;
    static OPUS_BUNDLED: std::sync::Once = std::sync::Once::new();
    OPUS_BUNDLED.call_once(|| {
        #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
        let lib = Some(("opus.dll", horizon_client::native_data!("voicechat", "opus-windows-x86_64.dll").as_slice()));
        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        let lib = Some(("libopus.dylib", horizon_client::native_data!("voicechat", "opus-macos-x86_64.dylib").as_slice()));
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        let lib = Some(("libopus.dylib", horizon_client::native_data!("voicechat", "opus-macos-aarch64.dylib").as_slice()));
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        let lib = Some(("libopus.so.0", horizon_client::native_data!("voicechat", "opus-linux-x86_64.so").as_slice()));
        #[cfg(not(any(
            all(target_os = "windows", target_arch = "x86_64"),
            all(target_os = "macos", target_arch = "x86_64"),
            all(target_os = "macos", target_arch = "aarch64"),
            all(target_os = "linux", target_arch = "x86_64"),
        )))]
        let lib: Option<(&str, &[u8])> = None;
        vc::opus::register_bundled(lib);
    });
    let on_server = game.remote.as_ref().is_some_and(|r| r.joined);
    if !on_server || !game.native_on(vc::ID) {
        vc::tick(None, &|_| None);
    }
    if vc::menu_take_close() {
        game.native_mod_screen_close(vc::ID);
    }
    vc::menu_sync(game.native_mod_screen() == Some(vc::ID));
}
