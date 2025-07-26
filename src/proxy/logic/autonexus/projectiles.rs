use super::{InflictedCondition, take_damage};
use crate::proxy::{Proxy, logic::common::bullets::BulletId};
use anyhow::{Result, bail};

pub async fn player_hit(proxy: &mut Proxy, bullet_id: u16, owner_id: u32) -> Result<()> {
	let bullet = match proxy.state.common.bullets.cache.get(&BulletId {
		id: bullet_id,
		owner_id,
	}) {
		Some(x) => *x,
		None => bail!("Player claims that he got hit by bullet which is not visible."),
	};

	take_damage(
		proxy,
		bullet.damage as i64,
		bullet.get_properties(proxy)?.armor_piercing,
	)
	.await;

	// immediatelly apply any status effects (conditions) if this bullet inflicts
	proxy.state.autonexus.inflicted_conditions.extend(
		bullet
			.get_properties(proxy)?
			.inflicts
			.clone()
			.iter()
			.map(|c| InflictedCondition {
				condition: c.condition,
				condition2: c.condition2,
				expires_in: (c.duration * 1000.0) as u32,
			}),
	);

	Ok(())
}
