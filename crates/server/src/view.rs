use bumpalo::Bump;
use common::{
    entity::player::{Username, View},
    ChunkPos, Pos, System, SystemExecutor,
};
use hashbrown::HashSet;
use hecs::Entity;
use protocol::packets::{
    server::{LoadChunk, UnloadChunk},
    ServerPacket,
};

use crate::{game::Game, Mailbox};

pub fn setup(systems: &mut SystemExecutor<Game>) {
    systems.add(ViewSystem::default());
}

/// System to
/// 1) update player's view when they move into a new chunk
/// 2) send new chunks when the view changes
/// 3) unload all chunks when the view changes
#[derive(Default)]
struct ViewSystem {
    known_players: HashSet<Entity>,
}

impl System<Game> for ViewSystem {
    fn run(&mut self, game: &mut Game) {
        let players = update_views(game, &mut self.known_players);
        update_chunks(&players, game);
    }
}

type UpdatedView = (Entity, View, View);

fn update_views<'g>(
    game: &'g Game,
    known_players: &mut HashSet<Entity>,
) -> Vec<UpdatedView, &'g Bump> {
    let mut updated = Vec::new_in(game.bump());
    let mut players_this_tick = HashSet::new_in(game.bump());

    for (player, (&pos, view)) in game.ecs().query::<(&Pos, &mut View)>().iter() {
        let chunk = ChunkPos::from_pos(pos);
        if chunk != view.center() {
            let old_view = *view;
            *view = View::new(chunk, view.distance());
            updated.push((player, old_view, *view));
        } else if !known_players.contains(&player) {
            // Player just joined, so we'll update their view.
            let old_view = View::empty();
            let new_view = *view;
            updated.push((player, old_view, new_view));
        }

        players_this_tick.insert(player);
    }

    known_players.clear();
    known_players.extend(players_this_tick);

    updated
}

fn update_chunks(players: &[UpdatedView], game: &Game) {
    for &(player, old_view, new_view) in players {
        // Consider using an analytical approach instead of brute forcing with sets
        let mut old_chunks = HashSet::new_in(game.bump());
        old_chunks.extend(old_view.iter());
        let mut new_chunks = HashSet::new_in(game.bump());
        new_chunks.extend(new_view.iter());

        let mut chunks_to_load = Vec::new_in(game.bump());
        chunks_to_load.extend(new_chunks.difference(&old_chunks));
        // Send closest chunks first.
        chunks_to_load.sort_by_key(|chunk: &ChunkPos| chunk.manhattan_distance(new_view.center()));

        let mailbox = game.ecs().get::<Mailbox>(player).unwrap();
        let username = game.ecs().get::<Username>(player).unwrap();

        for chunk_to_load in chunks_to_load {
            if let Some(chunk) = game.main_zone().chunk(chunk_to_load) {
                let packet = ServerPacket::LoadChunk(LoadChunk {
                    pos: chunk_to_load,
                    chunk: chunk.clone(),
                });
                log::trace!("Loading {:?} for {}", chunk_to_load, username.0);
                mailbox.send(packet);
            }
        }

        for &chunk_to_unload in old_chunks.difference(&new_chunks) {
            let packet = ServerPacket::UnloadChunk(UnloadChunk {
                pos: chunk_to_unload,
            });
            log::trace!("Unloading {:?} for {}", chunk_to_unload, username.0);
            mailbox.send(packet);
        }
    }
}
