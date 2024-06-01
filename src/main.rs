use anyhow::{bail, Context, Result};
use hex::FromHex;
use proxy::Proxy;
use std::collections::BTreeMap;
use std::io::{BufRead, ErrorKind, Read};
use std::sync::Arc;
use tokio::io::ErrorKind::WouldBlock;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::select;
use tokio::sync::Notify;

mod iptables;
mod packets;
mod proxy;
mod read;
mod rotmguard;
mod write;

#[tokio::main]
async fn main() -> Result<()> {
    // start by creating an iptables rule to redirect *:2050 (default rotmg port)
    // traffic to 127.0.0.1:2051
    let _iptables_rule = iptables::IpTablesRule::create()?;

    // Set up exit notify structure for shutting down clean
    let exit = Arc::new(Notify::new());
    let exit_clone = Arc::clone(&exit);
    ctrlc::set_handler(move || exit_clone.notify_waiters()).expect("Error setting Ctrl-C handler");

    select! {
        res = server() => res,
        _ = exit.notified() => { println!("Exiting..."); Ok(()) }
    }
}

async fn server() -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:2051").await?;

    loop {
        let (socket, _) = listener.accept().await?;

        // linux shenanigans 🤓
        // basically, since the connection was forwarded to ourselves using iptables, we need to obtain
        // the original destination address so we can connect to it
        let original_dst = std::net::IpAddr::from(
            nix::sys::socket::getsockopt(&socket, nix::sys::socket::sockopt::OriginalDst)?
                .sin_addr
                .s_addr
                .to_le_bytes(),
        );

        println!("Connecting to {original_dst}");
        let real_server = TcpStream::connect((original_dst, 2051)).await?;

        let mut proxy = Proxy::new(socket, real_server);

        if let Err(e) = proxy.run().await {
            if e.kind() != ErrorKind::UnexpectedEof {
                println!("ERROR: {e}");
            }
        }
    }
}
