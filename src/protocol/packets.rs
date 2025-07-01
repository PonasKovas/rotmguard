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
		reconnect -> Reconnect,
	}
}

// wraps the parsing function in another function that adds some context to any error
macro_rules! with_context {
    (
    	$context:literal;
    	pub fn parse($bytes:ident: &mut Bytes) -> Result<$return:ty, RPReadError> $code:tt
    ) => {
        pub fn parse($bytes: &mut Bytes) -> Result<$return, RPReadError> {
			fn parse_inner($bytes: &mut Bytes) -> Result<$return, RPReadError> $code

			parse_inner($bytes).map_err(|e| RPReadError::WithContext {
				ctx: $context.to_owned(),
				inner: Box::new(e),
			})
		}
    };
}
pub(crate) use with_context;
