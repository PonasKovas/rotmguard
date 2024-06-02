use crate::{
    extra_datatypes::{StatData, StatType},
    packets::{ClientPacket, EnemyShoot, Notification, ServerPacket},
    proxy::Proxy,
};
use anyhow::{Context, Result};
use rand::prelude::*;
use std::{collections::BTreeMap, time::Instant};

pub struct RotmGuard {
    max_hp: i64,
    hp: i64,
    def: i64,
    last_hit_instant: Instant, // the time instant when last hit was taken
    my_object_id: u32,
    my_name: String,
    bullets: BTreeMap<u16, EnemyShoot>, // bullet_id, bullet_info
}

impl RotmGuard {
    pub fn new() -> Self {
        Self {
            max_hp: 99999999,
            hp: 1,
            def: 0,
            last_hit_instant: Instant::now(),
            my_object_id: 0, // CreateSuccess packet sets this
            my_name: "?".to_owned(),
            bullets: BTreeMap::new(),
        }
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

                    let (r, g, b) = (
                        (r as u32 + 100).min(255),
                        (g as u32 + 100).min(255),
                        (b as u32 + 100).min(255),
                    );
                    let packet = Notification::Behavior {
                        message: format!("hi {} :)", proxy.rotmguard.my_name),
                        picture_type: 0,
                        color: (r << 16) | (g << 8) | (b << 0),
                    };
                    proxy.send_client(&packet.into()).await?;
                    return Ok(false); // dont forward this :)
                }
            }
            ClientPacket::PlayerHit(player_hit) => {
                proxy.rotmguard.last_hit_instant = Instant::now();

                let bullet_info =
                    proxy
                        .rotmguard
                        .bullets
                        .get(&player_hit.bullet_id)
                        .context(format!(
                            "Player got hit by bullet id {} which is not registered. All registered bullets: {:?}",
                            player_hit.bullet_id,
                            proxy
                                .rotmguard
                                .bullets
                        ))?;

                let damage = (bullet_info.damage as i64 - proxy.rotmguard.def)
                    .max(bullet_info.damage as i64 / 10);

                proxy.rotmguard.hp -= damage;

                println!("{} damage taken, {} hp left", damage, proxy.rotmguard.hp);

                if proxy.rotmguard.hp <= 0 {
                    // AUTONEXUS ENGAGE!!!
                    proxy.send_server(&ClientPacket::Escape).await?;
                    return Ok(false); // dont forward!!
                }

                let packet = Notification::ServerMessage {
                    text: format!("Damage {}", damage),
                };
                proxy.send_client(&packet.into()).await?;
            }
            _ => {}
        }

        Ok(true)
    }

    // True to forward packet, false to block
    pub async fn handle_server_packet(proxy: &mut Proxy, packet: &ServerPacket) -> Result<bool> {
        match packet {
            ServerPacket::EnemyShoot(enemy_shoot) => {
                for i in 0..=enemy_shoot.numshots {
                    proxy
                        .rotmguard
                        .bullets
                        .insert(enemy_shoot.bullet_id + i as u16, *enemy_shoot);
                }
            }
            ServerPacket::CreateSuccess(create_success) => {
                proxy.rotmguard.my_object_id = create_success.object_id;
            }
            ServerPacket::NewTick(new_tick) => {
                let mut new_tick = new_tick.clone();
                for status in &mut new_tick.statuses {
                    if status.object_id == proxy.rotmguard.my_object_id as i64 {
                        // remove MP and MAX MAP stats if there are
                        status.stats = status
                            .stats
                            .iter()
                            .cloned()
                            .filter(|stat| stat.stat_type != 3 && stat.stat_type != 4)
                            .collect();

                        // And add our own
                        status.stats.push(StatData {
                            stat_type: 3,
                            stat: StatType::Int(proxy.rotmguard.max_hp),
                            secondary_stat: -1,
                        });
                        status.stats.push(StatData {
                            stat_type: 4,
                            stat: StatType::Int(proxy.rotmguard.hp),
                            secondary_stat: -1,
                        });

                        for stat in &mut status.stats {
                            if stat.stat_type == 0 {
                                if let StatType::Int(val) = &stat.stat {
                                    proxy.rotmguard.max_hp = *val;
                                }

                                println!("MAX HP {:?}", stat.stat);
                            } else if stat.stat_type == 1 {
                                if proxy.rotmguard.last_hit_instant.elapsed().as_secs_f32() > 1.0 {
                                    if let StatType::Int(val) = &stat.stat {
                                        proxy.rotmguard.hp = *val;
                                    }
                                }

                                println!("HP {:?}", stat.stat);
                            } else if stat.stat_type == 21 {
                                if let StatType::Int(val) = &stat.stat {
                                    proxy.rotmguard.def = *val;
                                }
                                println!("DEF {:?}", stat.stat);
                            } else if stat.stat_type == 26 {
                                println!("VIT {:?}", stat.stat);
                            }
                        }
                    }
                }

                proxy.send_client(&new_tick.into()).await?;

                return Ok(false);
            }
            ServerPacket::UpdatePacket(update) => {
                for object in &update.new_objects {
                    if object.1.object_id == proxy.rotmguard.my_object_id as i64 {
                        for stat in &object.1.stats {
                            if stat.stat_type == 31 {
                                if let StatType::String(name) = &stat.stat {
                                    proxy.rotmguard.my_name = name.clone();
                                }
                            } else if stat.stat_type == 0 {
                                if let StatType::Int(val) = &stat.stat {
                                    proxy.rotmguard.max_hp = *val;
                                }
                            } else if stat.stat_type == 1 {
                                if let StatType::Int(val) = &stat.stat {
                                    proxy.rotmguard.hp = *val;
                                }
                            }
                        }
                    }
                }
            }
            ServerPacket::Aoe(aoe) => {
                println!("{aoe:?}");
            }
            ServerPacket::Unknown { id: 46 } => {
                println!("DEAD. Client HP at time of death: {}", proxy.rotmguard.hp);
            }
            _ => {}
        }

        Ok(true)
    }
}
