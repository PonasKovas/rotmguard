use super::RPReadError;
use bytes::{Buf, Bytes};

macro_rules! gen_enum {
	($enum_name:ident {
    	$($mod_name:ident -> $packet_name:ident),* $(,)?
    }) => {
    	$(
    		pub mod $mod_name;
    	)*
		pub enum $enum_name {
			$(
				$packet_name($mod_name::$packet_name),
			)*
		}
		impl $enum_name {
			pub fn parse(bytes: &mut Bytes) -> Result<Option<Self>, RPReadError> {
				let packet_id = bytes.get_u8();

				let parsed = match packet_id {
					$(
						$mod_name::$packet_name::ID => {
							let packet = $mod_name::$packet_name::parse(bytes)?;

							Some(Self::$packet_name(packet))
						},
					)*
					_ => None,
				};

				Ok(parsed)
			}
		}
    };
}

gen_enum! {
	C2SPacket {
		playertext -> PlayerText,
	}
}

gen_enum! {
	S2CPacket {
		notification -> Notification,
	}
}
