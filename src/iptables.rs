use anyhow::{Context, Result, bail};
use std::{
	env,
	net::UdpSocket,
	process::{Command, exit},
	time::Duration,
};
use tracing::{error, info};

// when this binary is run with this flag it will just add the iptables rules
pub const IPTABLES_ACTOR_FLAG: &str = "--iptables";

const TIMEOUT: u64 = 30; // seconds
const OK_SIGNAL: &[u8] = b"ok"; // 💀
const STOP_SIGNAL: &[u8] = b"stop";

pub fn iptables_actor() -> Result<()> {
	fn run(command: &str) -> Result<()> {
		let mut c = command.split(' ');

		let status = Command::new(c.next().unwrap()).args(c).status()?;
		if !status.success() {
			bail!("Unsuccessful: {command:?}");
		}

		Ok(())
	}

	let port: u16 = env::args().nth(2).context("udp port for status")?.parse()?;
	let socket = UdpSocket::bind("127.0.0.1:0")?;
	socket.set_read_timeout(None)?;
	socket.connect(("127.0.0.1", port))?;

	ctrlc::set_handler(|| {
		cleanup();
		exit(0);
	})?;

	struct Guard;
	impl Drop for Guard {
		fn drop(&mut self) {
			cleanup();
		}
	}
	let _guard = Guard;

	run("iptables -t nat -A OUTPUT -p tcp --dport 2050 -j DNAT --to-destination 127.0.0.1:2051")?;
	run("iptables -t nat -A OUTPUT -p tcp --dport 2051 -j DNAT --to-destination :2050")?;

	fn cleanup() {
		let _ = run(
			"iptables -t nat -D OUTPUT -p tcp --dport 2050 -j DNAT --to-destination 127.0.0.1:2051",
		);
		let _ = run("iptables -t nat -D OUTPUT -p tcp --dport 2051 -j DNAT --to-destination :2050");
	}

	socket.send(OK_SIGNAL)?;

	let mut buf = [0u8; STOP_SIGNAL.len()];
	socket.recv(&mut buf)?;
	if buf != STOP_SIGNAL {
		bail!("invalid signal received. stopping");
	}

	Ok(())
}

pub struct IpTablesRules {
	socket: UdpSocket,
}

impl IpTablesRules {
	pub fn create() -> Result<Self> {
		// for signaling OK status
		let socket = UdpSocket::bind("127.0.0.1:0")?;
		socket.set_read_timeout(Some(Duration::from_secs(TIMEOUT)))?;
		let port = socket.local_addr()?.port();

		let exe = env::current_exe()?;
		Command::new("sudo")
			.arg("-b")
			.arg(exe)
			.arg(IPTABLES_ACTOR_FLAG)
			.arg(format!("{port}"))
			.spawn()?;

		let mut buf = [0u8; OK_SIGNAL.len()];
		let (_, origin) = socket.recv_from(&mut buf)?;
		socket.connect(origin)?;

		if buf == OK_SIGNAL {
			info!("IPTables rule created successfully.");
		} else {
			error!("Received invalid signal from iptables actor");

			socket.send(STOP_SIGNAL)?;

			bail!("Error creating iptables rules");
		}

		Ok(Self { socket })
	}
}

impl Drop for IpTablesRules {
	fn drop(&mut self) {
		info!("Cleaning up IP tables rules.");
		let _ = self.socket.send(STOP_SIGNAL);
	}
}
