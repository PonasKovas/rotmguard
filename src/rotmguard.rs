use crate::{
    packets::{ClientPacket, Notification, ServerPacket},
    proxy::Proxy,
};
use rand::prelude::*;
use std::io::Result;
use tokio::io::AsyncWriteExt;

pub struct RotmGuard {}

impl RotmGuard {
    pub fn new() -> Self {
        Self {}
    }
    // True to forward packet, false to block
    pub async fn handle_client_packet(proxy: &mut Proxy, packet: &ClientPacket) -> Result<bool> {
        match packet {
            ClientPacket::PlayerText(player_text) => {
                if player_text.text == "/hi" {
                    fn hue_to_rgb(hue: f64) -> (u8, u8, u8) {
                        let c = 1.0;
                        let x = c * (1.0 - ((hue / 60.0) % 2.0 - 1.0).abs());

                        let (r, g, b) = match hue {
                            0.0..=60.0 => (c, x, 0.0),
                            60.0..=120.0 => (x, c, 0.0),
                            120.0..=180.0 => (0.0, c, x),
                            180.0..=240.0 => (0.0, x, c),
                            240.0..=300.0 => (x, 0.0, c),
                            300.0..=360.0 => (c, 0.0, x),
                            _ => (0.0, 0.0, 0.0),
                        };

                        let r = (r * 255.0).round() as u8;
                        let g = (g * 255.0).round() as u8;
                        let b = (b * 255.0).round() as u8;

                        (r, g, b)
                    }
                    let hue = rand::thread_rng().gen_range(0.0..360.0);
                    let (r, g, b) = hue_to_rgb(hue);

                    let packet = Notification::Behavior {
                        message: "hi :)".to_owned(),
                        picture_type: 0,
                        color: ((r as u32) << 16) | ((g as u32) << 8) | ((b as u32) << 0),
                    };
                    proxy.send_client(&packet.into()).await?;
                    return Ok(false); // dont forward this :)
                }
            }
            _ => {}
        }

        Ok(true)
    }

    // True to forward packet, false to block
    pub async fn handle_server_packet(proxy: &mut Proxy, packet: &ServerPacket) -> Result<bool> {
        match packet {
            _ => {}
        }

        Ok(true)
    }
}
