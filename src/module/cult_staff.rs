use super::{Module, ModuleInstance, PacketFlow, BLOCK, FORWARD};
use crate::{
	gen_this_macro,
	packets::{ClientPacket, PlayerShoot},
	proxy::Proxy,
};
use anyhow::Result;
use std::f32::consts::PI;

gen_this_macro! {cult_staff}

#[derive(Debug, Clone)]
pub struct CultStaff {}

#[derive(Debug, Clone)]
pub struct CultStaffInst {
	packets: Vec<PlayerShoot>,
}

impl Module for CultStaff {
	type Instance = CultStaffInst;

	fn new() -> Self {
		CultStaff {}
	}
	fn instance(&self) -> Self::Instance {
		CultStaffInst {
			packets: Vec::with_capacity(4),
		}
	}
}

impl ModuleInstance for CultStaffInst {
	async fn client_packet<'a>(
		proxy: &mut Proxy<'_>,
		packet: &mut ClientPacket<'a>,
	) -> Result<PacketFlow> {
		if !proxy.config.settings.edit_assets.enabled
			|| !proxy.config.settings.edit_assets.cult_staff
		{
			return FORWARD;
		}

		if let ClientPacket::PlayerShoot(player_shoot) = packet {
			if player_shoot.weapon_id as u32 != proxy.assets.cult_staff_id {
				// only care about cult staff
				return FORWARD;
			}

			cult_staff!(proxy).packets.push(player_shoot.clone());

			if cult_staff!(proxy).packets.len() == 4 {
				for (i, packet) in cult_staff!(proxy).packets.iter_mut().rev().enumerate() {
					// Make it seem like the angles between the shots are 345 degrees instead of 15
					packet.angle += 2.0 * PI * (i as f32);

					proxy.write.send_server(&(*packet).into()).await?;
				}

				cult_staff!(proxy).packets.clear();
			}

			return BLOCK;
		}

		FORWARD
	}
}
