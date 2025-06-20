use crate::proxy::Proxy;
use phf::phf_map;

pub static SERVERS: phf::Map<&str, &str> = phf_map! {
	"eue"	=> "18.184.218.174",
	"eusw"	=> "35.180.67.120",
	"use2"	=> "54.209.152.223",
	"eun"	=> "18.159.133.120",
	"use"	=> "54.234.226.24",
	"usw4"	=> "54.235.235.140",
	"euw2"	=> "52.16.86.215",
	"a"		=> "3.0.147.127",
	"uss3"	=> "52.207.206.31",
	"euw"	=> "15.237.60.223",
	"usw"	=> "54.86.47.176",
	"usmw2"	=> "3.140.254.133",
	"usmw"	=> "18.221.120.59",
	"uss"	=> "3.82.126.16",
	"usw3"	=> "18.144.30.153",
	"ussw"	=> "54.153.13.68",
	"usnw"	=> "34.238.176.119",
	"aus"	=> "54.79.72.84"
};

pub fn con<'a>(proxy: &mut Proxy, mut args: impl Iterator<Item = &'a str>) {
	let server = match args.next() {
		Some(s) => s,
		None => {
			// todo
			return;
		}
	};

	if args.count() > 0 {
		// todo
		return;
	}

	match SERVERS.get(server) {
		Some(ip) => {}
		None => todo!(),
	}
}
