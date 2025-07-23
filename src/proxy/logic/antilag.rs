//! this basically blocks the packets that have no effect if the client has "ally damage" and "ally notifications"
//! disabled but still makes the client lag af..

use crate::proxy::Proxy;

// for development purpoes
pub static BLOCK_TYPE: std::sync::Mutex<u8> = std::sync::Mutex::new(0);

pub fn should_block_damage(proxy: &mut Proxy, bullet_owner_obj_id: u32) -> bool {
	if *BLOCK_TYPE.lock().unwrap() & 1 == 0 {
		return false;
	}

	// block if antilag enabled and if the damage was not caused by me

	// (we wanna see our own damage!!)

	*proxy.rotmguard.config.settings.antilag.lock().unwrap()
		&& bullet_owner_obj_id != proxy.state.my_obj_id
}

pub fn should_block_object_notification(
	proxy: &mut Proxy,
	obj_id: u32,
	_color: u32,
	_message: &str,
) -> bool {
	if *BLOCK_TYPE.lock().unwrap() & 2 == 0 {
		return false;
	}

	// block if antilag enabled and if the notification is not on me

	*proxy.rotmguard.config.settings.antilag.lock().unwrap() && obj_id != proxy.state.my_obj_id
}
