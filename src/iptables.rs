use anyhow::{Context, Result, bail};
use std::{
	env,
	net::UdpSocket,
	process::{Command, exit},
	time::Duration,
};
use tracing::info;

// when this binary is run with this flag it will just add the iptables rules
pub const IPTABLES_ACTOR_FLAG: &str = "--iptables";

const TIMEOUT: u64 = 5; // seconds

#[derive(PartialEq)]
#[repr(u8)]
enum Signal {
	// actor to main:
	A2MStarted = 0,
	A2MOk,
	// main to actor
	M2AStop,
}

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
	socket.send(&[Signal::A2MStarted as u8])?;

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

	socket.send(&[Signal::A2MOk as u8])?;

	let mut reply = [0u8; 1];
	socket.recv(&mut reply)?;
	if reply[0] != Signal::M2AStop as u8 {
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
		socket.set_read_timeout(None)?;
		let port = socket.local_addr()?.port();

		let exe = env::current_exe()?;
		Command::new("sudo")
			.arg("-b") // if we dont use this, sudo fucks up the terminal mode
			.arg(exe)
			.arg(IPTABLES_ACTOR_FLAG)
			.arg(format!("{port}"))
			.spawn()?;

		let mut reply = [0u8; 1];
		let (_, origin) = socket.recv_from(&mut reply)?;
		if reply[0] != Signal::A2MStarted as u8 {
			bail!("Invalid signal received");
		}

		socket.connect(origin)?;
		socket.set_read_timeout(Some(Duration::from_secs(TIMEOUT)))?;

		socket.recv(&mut reply)?;
		if reply[0] != Signal::A2MOk as u8 {
			socket.send(&[Signal::M2AStop as u8])?;

			bail!("Error creating iptables rules. Invalid signal received");
		}

		info!("IPTables rule created successfully.");

		Ok(Self { socket })
	}
}

impl Drop for IpTablesRules {
	fn drop(&mut self) {
		info!("Cleaning up IP tables rules.");
		let _ = self.socket.send(&[Signal::M2AStop as u8]);
	}
}
