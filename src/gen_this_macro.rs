// convenience macro for accessing a specific module
#[macro_export]
macro_rules! gen_this_macro {
	($name:ident) => {
		macro_rules! $name {
			($proxy:expr) => {
				$proxy.modules.$name
			};
		}
	};
	($first:ident . $name:ident) => {
		macro_rules! $name {
			($proxy:expr) => {
				$proxy.modules.$first.$name
			};
		}
	};
	($first:ident . $second:ident . $name:ident) => {
		macro_rules! $name {
			($proxy:expr) => {
				$proxy.modules.$first.$second.$name
			};
		}
	};
}
