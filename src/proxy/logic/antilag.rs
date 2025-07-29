//! this basically blocks the packets that have no effect if the client has "ally damage" and "ally notifications"
//! disabled but still makes the client lag af..

use crate::proxy::Proxy;

pub fn should_block_damage(
	proxy: &mut Proxy,
	target_obj_id: u32,
	bullet_owner_obj_id: u32,
) -> bool {
	// block if antilag enabled and if the damage was not caused by me or to me

	// (we wanna see our own damage!!)

	if *proxy.rotmguard.config.settings.antilag.lock().unwrap() {
		let self_id = proxy.state.common.objects.self_id;

		if bullet_owner_obj_id == self_id || target_obj_id == self_id {
			return false;
		} else {
			return true;
		}
	}
	false
}

pub fn should_block_object_notification(
	proxy: &mut Proxy,
	obj_id: u32,
	_color: u32,
	_message: &str,
) -> bool {
	// block if antilag enabled and if the notification is on another player which is not me

	let self_id = proxy.state.common.objects.self_id;
	let obj = match proxy.state.common.objects.get(obj_id) {
		Some(x) => x,
		None => return false, // if we are not aware of the object, safer to let it pass
	};

	if obj_id != self_id && obj.is_player {
		*proxy.rotmguard.config.settings.antilag.lock().unwrap()
	} else {
		false
	}
}
