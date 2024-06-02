use crate::{
    asset_extract,
    extra_datatypes::{ObjectStatusData, Stat, StatData, StatType, WorldPos},
    packets::{AoePacket, ClientPacket, EnemyShoot, GotoPacket, Notification, ServerPacket},
    proxy::Proxy,
};
use anyhow::{bail, Context, Result};
use rand::prelude::*;
use std::{
    collections::{BTreeMap, HashMap},
    time::Instant,
};

const MAX_HP_STAT: u8 = 0;
const HP_STAT: u8 = 1;
const MAX_MP_STAT: u8 = 3;
const MP_STAT: u8 = 4;
const DEF_STAT: u8 = 21;
const VIT_STAT: u8 = 26;
const NAME_STAT: u8 = 31;

#[derive(Debug, Clone)]
pub struct RotmGuard {
    // the simulated HP of the player
    hp: i64,
    // the time instant when last hit was taken
    last_hit_instant: Instant,
    // the player's object id
    my_object_id: i64,
    // the player's username
    my_name: String,
    // all currently visible bullets
    bullets: BTreeMap<u16, Bullet>,
    // maps the object id of a currently visible object to it's type id
    objects: BTreeMap<i64, u16>,
    // all important stats of the player
    player_stats: PlayerStats,
    // all once seen ground tiles that could deal damage. Map<(x, y) -> damage>
    hazardous_tiles: HashMap<(i16, i16), i64>,
}

#[derive(Debug, Clone, Copy)]
pub struct Bullet {
    pub damage: i16,
    pub armor_piercing: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct PlayerStats {
    max_hp: i64,
    def: i64,
    vit: i64,
}

impl RotmGuard {
    pub fn new() -> Self {
        Self {
            hp: 1,
            last_hit_instant: Instant::now(),
            my_object_id: 0, // CreateSuccess packet sets this
            my_name: "?".to_owned(),
            bullets: BTreeMap::new(),
            objects: BTreeMap::new(),
            player_stats: PlayerStats {
                max_hp: 0,
                def: 0,
                vit: 0,
            },
            hazardous_tiles: HashMap::new(),
        }
    }
    // True to forward packet, false to block
    pub async fn handle_client_packet(proxy: &mut Proxy, packet: &ClientPacket) -> Result<bool> {
        match packet {
            ClientPacket::PlayerText(player_text) => {
                if player_text.text == "/hi" {
                    let colors = [0xff8080, 0xff8080, 0x80ffac, 0x80c6ff, 0xc480ff];
                    let color = colors[rand::thread_rng().gen_range(0..colors.len())];

                    let packet = Notification::Behavior {
                        message: format!("hi {} :)", proxy.rotmguard.my_name),
                        picture_type: 0,
                        color,
                    };
                    proxy.send_client(&packet.into()).await?;
                    return Ok(false); // dont forward this :)
                }
            }
            ClientPacket::PlayerHit(player_hit) => {
                proxy.rotmguard.last_hit_instant = Instant::now();

                let bullet_info = match proxy.rotmguard.bullets.get(&player_hit.bullet_id) {
                    Some(info) => *info,
                    None => {
                        println!(
                            "Player claims that he got hit by bullet id {} which is not visible.",
                            player_hit.bullet_id,
                        );
                        println!("PlayerHit: {player_hit:?}");

                        return Ok(false); // Dont forward the packet then, better get DCed than die.
                    }
                };

                let damage = if bullet_info.armor_piercing {
                    bullet_info.damage as i64
                } else {
                    let def = proxy.rotmguard.player_stats.def;
                    (bullet_info.damage as i64 - def).max(bullet_info.damage as i64 / 10)
                };

                return RotmGuard::take_damage(proxy, damage).await;
            }
            ClientPacket::GroundDamage(ground_damage) => {
                let x = ground_damage.position.x as i16;
                let y = ground_damage.position.y as i16;

                let damage = match proxy.rotmguard.hazardous_tiles.get(&(x, y)) {
                    Some(damage) => damage,
                    None => {
                        println!("Player claims to take ground damage when not standing on hazardous ground! Maybe your assets are outdated?");
                        println!("Nexusing");

                        proxy.send_server(&ClientPacket::Escape).await?;
                        return Ok(false);
                    }
                };

                return RotmGuard::take_damage(proxy, *damage).await;
            }
            _ => {}
        }

        Ok(true)
    }

    // True to forward packet, false to block
    pub async fn handle_server_packet(proxy: &mut Proxy, packet: &ServerPacket) -> Result<bool> {
        match packet {
            ServerPacket::EnemyShoot(enemy_shoot) => {
                // check if the bullet is armor piercing
                let shooter_id = enemy_shoot.owner_id as i64;
                let shooter_object_type = match proxy.rotmguard.objects.get(&shooter_id) {
                    Some(object_type) => *object_type as u32,
                    None => {
                        // this happens all the time, server sends info about bullets that are not even in visible range
                        // its safe to assume that the client ignores these too
                        return Ok(true);
                    }
                };

                let projectiles_assets_lock = asset_extract::PROJECTILES.lock().unwrap();

                let shooter_projectile_types = match projectiles_assets_lock
                    .get(&shooter_object_type)
                {
                    Some(types) => types,
                    None => {
                        println!("Bullet shot by enemy of which assets are not registered. Maybe your assets are outdated?");
                        println!("EnemyShoot: {enemy_shoot:?}");

                        return Ok(false); // i guess dont forward the packet, better get DCed than die
                    }
                };

                let armor_piercing = match shooter_projectile_types
                    .get(&(enemy_shoot.bullet_type as u32))
                {
                    Some(piercing) => *piercing,
                    None => {
                        println!("Bullet type shot of which assets are not registered. Maybe your assets are outdated?");
                        println!("EnemyShoot: {enemy_shoot:?}");

                        return Ok(false); // i guess dont forward the packet, better get DCed than die
                    }
                };

                // create N bullets with incremental IDs where N is the number of shots
                for i in 0..=enemy_shoot.numshots {
                    proxy.rotmguard.bullets.insert(
                        enemy_shoot.bullet_id + i as u16,
                        Bullet {
                            damage: enemy_shoot.damage,
                            armor_piercing,
                        },
                    );
                }
            }
            ServerPacket::CreateSuccess(create_success) => {
                proxy.rotmguard.my_object_id = create_success.object_id as i64;
            }
            // This packet only adds/removes new objects, doesnt update existing ones
            ServerPacket::Update(update) => {
                // remove objects that left the visible area
                for object in &update.to_remove {
                    proxy.rotmguard.objects.remove(object);
                }

                // Add new objects
                for object in &update.new_objects {
                    // handle my stats
                    if object.1.object_id == proxy.rotmguard.my_object_id {
                        for stat in &object.1.stats {
                            if stat.stat_type == StatType::Name {
                                proxy.rotmguard.my_name = stat.stat.as_str();
                            } else if stat.stat_type == StatType::HP {
                                proxy.rotmguard.hp = stat.stat.as_int();
                            } else if stat.stat_type == StatType::MaxHP {
                                proxy.rotmguard.player_stats.max_hp = stat.stat.as_int();
                            } else if stat.stat_type == StatType::Defense {
                                proxy.rotmguard.player_stats.def = stat.stat.as_int();
                            } else if stat.stat_type == StatType::Vitality {
                                proxy.rotmguard.player_stats.vit = stat.stat.as_int();
                            }
                        }
                    } else {
                        proxy.rotmguard.objects.insert(object.1.object_id, object.0);
                    }
                }

                // Add hazardous tiles if any
                let hazard_tile_register = asset_extract::HAZARDOUS_GROUNDS.lock().unwrap();
                for tile in &update.tiles {
                    match hazard_tile_register.get(&(tile.tile_type as u32)) {
                        Some(damage) => {
                            // Add the tile
                            proxy
                                .rotmguard
                                .hazardous_tiles
                                .insert((tile.x, tile.y), *damage);
                        }
                        None => {} // dont care about normal tiles
                    }
                }
            }
            // This packet updates existing objects
            ServerPacket::NewTick(new_tick) => {
                // We clone the packet so we can mutate it and forward a modified one instead of the original
                let mut new_tick = new_tick.clone();

                for status in &mut new_tick.statuses {
                    // Frankly we are only interested in ourselves
                    if status.object_id != proxy.rotmguard.my_object_id {
                        continue;
                    }

                    for stat in &mut status.stats {
                        if stat.stat_type == StatType::HP {
                            // Only sync HP with the server if no shots have been taken for 1 second straight
                            // to make sure they're actually in sync.
                            if proxy.rotmguard.last_hit_instant.elapsed().as_secs_f32() > 1.0 {
                                proxy.rotmguard.hp = stat.stat.as_int();
                            }
                        }
                    }

                    // remove MP and MAX MAP updates if there are
                    status
                        .stats
                        .retain(|s| s.stat_type != StatType::MaxMP && s.stat_type != StatType::MP);

                    // And add our own
                    status.stats.push(StatData {
                        stat_type: StatType::MaxMP,
                        stat: Stat::Int(proxy.rotmguard.player_stats.max_hp),
                        secondary_stat: -1,
                    });
                    status.stats.push(StatData {
                        stat_type: StatType::MP,
                        stat: Stat::Int(proxy.rotmguard.hp),
                        secondary_stat: -1,
                    });
                }

                proxy.send_client(&new_tick.into()).await?;

                return Ok(false);
            }
            ServerPacket::Unknown { id: 46 } => {
                println!("DEAD. Client HP at time of death: {}", proxy.rotmguard.hp);
            }
            _ => {}
        }

        Ok(true)
    }
    // Calculates real damage by considering status effects, modifies the client hp and nexuses if necessary
    // This does not consider DEFENSE. Give damage with defense already calculated
    pub async fn take_damage(proxy: &mut Proxy, damage: i64) -> Result<bool> {
        proxy.rotmguard.hp -= damage;

        println!("{} damage taken, {} hp left.", damage, proxy.rotmguard.hp);

        if proxy.rotmguard.hp <= 0 {
            // AUTONEXUS ENGAGE!!!
            proxy.send_server(&ClientPacket::Escape).await?;
            return Ok(false); // dont forward!!
        }

        let packet = Notification::Behavior {
            message: format!("DAMAGE {}", damage),
            picture_type: 0,
            color: 0x888888,
        };
        proxy.send_client(&packet.into()).await?;

        Ok(true)
    }
}
