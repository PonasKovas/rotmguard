use super::{Module, ModuleInstance, PacketFlow, BLOCK, FORWARD};
use crate::{
	gen_this_macro,
	packets::{ClientPacket, ServerPacket, TileData},
	proxy::Proxy,
	util::Notification,
};
use std::{collections::BTreeMap, io::Result};

gen_this_macro! {anti_push}

// the tile with which all pushing tiles are replaced when antipush enabled
const ANTI_PUSH_TILE: u16 = 0x2230; // Spider dirt ground, which reduces walking speed to 35%

#[derive(Debug, Clone)]
pub struct AntiPush {}

#[derive(Debug, Clone)]
pub struct AntiPushInst {
	// all tiles that push that were seen in this map
	tiles: BTreeMap<(i16, i16), u16>, // position -> original tile type
	enabled: bool,
	synced: bool,
}

impl Module for AntiPush {
	type Instance = AntiPushInst;

	fn new() -> Self {
		AntiPush {}
	}
	fn instance(&self) -> Self::Instance {
		AntiPushInst {
			tiles: BTreeMap::new(),
			enabled: false,
			synced: true,
		}
	}
}

impl ModuleInstance for AntiPushInst {
	async fn client_packet<'a>(
		proxy: &mut Proxy<'_>,
		packet: &mut ClientPacket<'a>,
	) -> Result<PacketFlow> {
		if let ClientPacket::PlayerText(text) = packet {
			let text = &text.text;
			// `/slow` toggles a permanent slow effect
			if text.starts_with("/ap") || text.starts_with("/antipush") {
				anti_push!(proxy).enabled = !anti_push!(proxy).enabled;
				let msg = if anti_push!(proxy).enabled {
					"Antipush enabled."
				} else {
					"Antipush disabled."
				};
				anti_push!(proxy).synced = false;

				Notification::new(msg.to_owned())
					.green()
					.send(&mut proxy.write)
					.await?;

				return BLOCK;
			}
		}

		FORWARD
	}
	async fn server_packet<'a>(
		proxy: &mut Proxy<'_>,
		packet: &mut ServerPacket<'a>,
	) -> Result<PacketFlow> {
		if let ServerPacket::Update(update) = packet {
			// sync antipush if not synced
			// that means update all tiles that were sent previously to remove push or to revert
			if !anti_push!(proxy).synced {
				for (&(x, y), &tile_type) in &anti_push!(proxy).tiles {
					if anti_push!(proxy).enabled {
						update.tiles.push(TileData {
							x,
							y,
							tile_type: ANTI_PUSH_TILE,
						});
					} else {
						update.tiles.push(TileData { x, y, tile_type });
					}
				}

				anti_push!(proxy).synced = true;
			}

			// Add pushing tiles if any are visible
			for tile in &mut update.tiles {
				let tile_type = tile.tile_type as u32;

				if proxy.assets.pushing_grounds.contains(&tile_type) {
					anti_push!(proxy)
						.tiles
						.insert((tile.x, tile.y), tile.tile_type);

					// if enabled replace the new tile
					if anti_push!(proxy).enabled {
						tile.tile_type = ANTI_PUSH_TILE;
					}
				}
			}
		}

		FORWARD
	}
}
