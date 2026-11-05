// What voice chat needs from the game that the native hook points don't give
// a client yet: the address the player joined, where the other players stand
// and the camera the world was drawn from. Everything else goes through the
// hook points. This module is the mod's only reach into the game's own state,
// so a Horizon that moves these fields breaks the build here and nowhere else.
// (A `native_client_players` / `native_server_address` hook would retire it.)
//
// Added to `game::remote` (`horizon-mixin.toml`, `[[module]]`), whose private
// fields its children can read.

/// On a server and in its world: joined, and not reconfiguring.
pub(crate) fn on_server(game: &crate::game::Game<'_>) -> bool {
    game.remote
        .as_ref()
        .is_some_and(|r| r.joined && r.reconfiguring.is_none())
}

/// The address the player joined, as typed (`host` or `host:port`).
pub(crate) fn address(game: &crate::game::Game<'_>) -> Option<String> {
    game.remote
        .as_ref()
        .filter(|_| on_server(game))
        .map(|r| r.session.address.clone())
}

/// The other players in the level: UUID and feet, from one pass over the mobs.
pub(crate) fn players(game: &crate::game::Game<'_>) -> Vec<(u128, [f64; 3])> {
    let Some(r) = game.remote.as_ref() else {
        return Vec::new();
    };
    if r.level_players.is_empty() {
        return Vec::new();
    }
    game.mobs
        .iter()
        .filter_map(|m| {
            let uuid = r.level_players.get(&i32::try_from(m.id).ok()?)?;
            Some((*uuid, [m.position.x, m.position.y, m.position.z]))
        })
        .collect()
}

/// The camera the world was last drawn from: position, yaw, pitch and field of view.
pub(crate) fn camera(game: &crate::game::Game<'_>) -> Option<([f64; 3], f32, f32, f32)> {
    game.locator_camera
}
