use crate::proxy::Proxy;

pub fn should_block_damage(proxy: &mut Proxy, bullet_owner_obj_id: u32) -> bool {
	// block if antilag enabled and if the damage was not caused by me
	// (we wanna see our own damage!!)
	*proxy.rotmguard.config.settings.antilag.lock().unwrap()
		&& bullet_owner_obj_id != proxy.state.my_obj_id
}
