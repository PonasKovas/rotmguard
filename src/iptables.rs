use anyhow::{Result, bail};
use nix::{sys::signal::Signal, unistd::Pid};
use std::{
	env,
	io::{Read, Write, stdout},
	process::{Command, Stdio, exit},
	thread::park,
};
use tracing::info;

// when this binary is run with this flag it will just add the iptables rules
pub const IPTABLES_ACTOR_FLAG: &str = "--iptables";
const OK_SIGNAL: &[u8] = b"ok";

pub fn iptables_actor() -> Result<()> {
	fn run(command: &str) -> Result<()> {
		let mut c = command.split(' ');

		let status = Command::new(c.next().unwrap()).args(c).status()?;
		if !status.success() {
			bail!("Unsuccessful: {command:?}");
		}

		Ok(())
	}

	ctrlc::set_handler(|| {
		cleanup();
		exit(0);
	})?;

	run("iptables -t nat -A OUTPUT -p tcp --dport 2050 -j DNAT --to-destination 127.0.0.1:2051")?;
	run("iptables -t nat -A OUTPUT -p tcp --dport 2051 -j DNAT --to-destination :2050")?;

	fn cleanup() {
		let _ = run(
			"iptables -t nat -D OUTPUT -p tcp --dport 2050 -j DNAT --to-destination 127.0.0.1:2051",
		);
		let _ = run("iptables -t nat -D OUTPUT -p tcp --dport 2051 -j DNAT --to-destination :2050");
	}

	// ignore all errors to guarantee cleanup
	{
		let mut stdout = stdout().lock();
		let _ = stdout.write_all(OK_SIGNAL);
		let _ = stdout.flush();
	}

	park();

	Ok(())
}

pub struct IpTablesRules {
	child_pid: Pid,
}

impl IpTablesRules {
	pub fn create() -> Result<Self> {
		let exe = env::current_exe()?;
		let mut child = Command::new("sudo")
			.arg(exe)
			.arg(IPTABLES_ACTOR_FLAG)
			.stdout(Stdio::piped())
			.spawn()?;

		let mut child_stdout = child.stdout.take().unwrap();
		let mut buf = [0u8; OK_SIGNAL.len()]; // buffer for "ok"
		child_stdout.read_exact(&mut buf)?;

		if buf == OK_SIGNAL {
			info!("IPTables rule created successfully.");
		} else {
			// Somethings wrong
			// wait for child to exit and forward it's stdout to my own
			child.wait()?;

			let mut stdout = stdout().lock();

			stdout.write_all(&buf)?;
			let mut remaining = Vec::new();
			child_stdout.read_to_end(&mut remaining)?;
			stdout.write_all(&remaining)?;

			bail!("Error creating iptables rules");
		}

		Ok(Self {
			child_pid: Pid::from_raw(child.id() as i32),
		})
	}
}

impl Drop for IpTablesRules {
	fn drop(&mut self) {
		let _ = nix::sys::signal::kill(self.child_pid, Signal::SIGTERM);
	}
}
