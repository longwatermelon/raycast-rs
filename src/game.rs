use crate::audio::Audio;
use raycast::prelude as rc;
use rc::prelude::{macroquad, glam};
use macroquad::prelude as mq;
use glam::{Vec2, IVec2};
use std::collections::HashMap;

const MAX_ENTS: usize = 15;
const MAX_AMMO: usize = 3;
const NUTS_GOAL: i32 = 5;

struct Entities {
    ents: Vec<rc::Entity>,
    speeds: Vec<f32>,
    death_timers: Vec<Option<f64>>,
}

impl Entities {
    fn new() -> Self {
        Self {
            ents: Vec::new(),
            speeds: Vec::new(),
            death_timers: Vec::new(),
        }
    }

    fn push(&mut self, ent: rc::Entity, speed: f32) {
        self.ents.push(ent);
        self.speeds.push(speed);
        self.death_timers.push(None);
    }

    fn remove(&mut self, index: usize) {
        self.ents.remove(index);
        self.speeds.remove(index);
        self.death_timers.remove(index);
    }
}

fn random_spot(map: &rc::Map) -> Vec2 {
    let mut res: Vec2 = Vec2::default();
    loop {
        res.x = mq::rand::gen_range(0., map.w * map.tsize);
        res.y = mq::rand::gen_range(0., map.h * map.tsize);

        let gpos: IVec2 = map.gpos(res);
        if map.at(gpos.x, gpos.y) == '.' {
            break;
        }
    }

    res
}

pub struct Game {
    audio: Audio,
}

impl Game {
    pub async fn new() -> Self {
        let audio: Audio = Audio::new().await;
        audio.loop_sound("music");

        Self {
            audio,
        }
    }

    pub async fn run(&self) {
        rc::util::set_scrw_scrh(800, 800);

        let mut textures: HashMap<char, mq::Image> = HashMap::new();
        textures.insert('0', mq::Image::from_file_with_format(include_bytes!("res/wall.png"), Some(mq::ImageFormat::Png)).unwrap());
        textures.insert('e', mq::Image::from_file_with_format(include_bytes!("res/shrek.png"), Some(mq::ImageFormat::Png)).unwrap());
        textures.insert('d', mq::Image::from_file_with_format(include_bytes!("res/shrek_dead_gun.png"), Some(mq::ImageFormat::Png)).unwrap());
        textures.insert('n', mq::Image::from_file_with_format(include_bytes!("res/deez.png"), Some(mq::ImageFormat::Png)).unwrap());
        textures.insert('a', mq::Image::from_file_with_format(include_bytes!("res/ammo.png"), Some(mq::ImageFormat::Png)).unwrap());
        let mut map: rc::Map = rc::Map::from_bytes(include_bytes!("res/map"), textures);
        map.floor_tex(rc::Surface::Color(mq::DARKGRAY.into()));
        map.ceil_tex(rc::Surface::Color(mq::GRAY.into()));

        let mut ents: Entities = Entities::new();
        let mut nut: Vec<rc::Entity> = Vec::new();
        let mut ammo_ents: Vec<rc::Entity> = Vec::new();

        let mut items: Vec<rc::Item> = vec![
            rc::Item::new("gun", include_bytes!("res/gun.png")),
        ];
        let mut item: usize = 0;
        rc::equip_item(&mut items, "gun");

        let shooting_gun: mq::Texture2D = mq::Texture2D::from_file_with_format(include_bytes!("res/gun-shoot.png"), Some(mq::ImageFormat::Png));

        let mut cam: rc::Ray = rc::Ray::new(Vec2::new(100., 100.), 0.);
        let mut prev_mpos: (f32, f32) = mq::mouse_position();

        let mut grabbed: bool = true;
        mq::set_cursor_grab(true);
        mq::show_mouse(false);

        let mut out_img: mq::Image = mq::Image::gen_image_color(
            rc::scrw() as u16,
            rc::scrh() as u16,
            mq::BLACK
        );
        let out_tex: mq::Texture2D = mq::Texture2D::from_image(&out_img);

        let mut grappling: bool = false;
        let mut grapple_target: Vec2 = Vec2::default();

        let mut ammo: i32 = 16;
        let mut inv_ammo: i32 = 32;
        let mut reload_start: Option<f64> = None;

        let mut health: i32 = 3;
        let mut last_hurt: f64 = -100.;

        let mut nuts_collected: i32 = 0;

        let mut shake_begin: f64 = -100.;

        let mut wallh: f32;

        loop {
            // wallh = mq::get_time().sin() as f32 + 3.;
            wallh = 2.;
            map.wall_height('0', wallh);
            if mq::is_key_pressed(mq::KeyCode::Escape) {
                grabbed = !grabbed;
                mq::set_cursor_grab(grabbed);
                mq::show_mouse(!grabbed);
            }

            if health > 0 && nuts_collected < NUTS_GOAL {
                // Movement
                if grappling {
                    if cam.orig.distance(grapple_target) < 20. {
                        grappling = false;
                        self.audio.play_sound("impact");
                        shake_begin = mq::get_time();
                    } else {
                        cam.orig = rc::util::move_towards_collidable(&map, cam.orig, grapple_target, 8.);
                    }
                } else {
                    rc::util::fps_camera_controls(&map, &mut cam, 2.);
                }
                rc::util::fps_camera_rotation(&mut cam, &mut prev_mpos, 0.5);

                // Misc keys
                if mq::is_key_pressed(mq::KeyCode::R) {
                    reload_start = Some(mq::get_time());
                    items[0].unequip();
                    self.audio.play_sound("reload");
                }

                // Reloading
                if let Some(start) = reload_start {
                    if mq::get_time() - start > 2. {
                        reload_start = None;
                        let reloaded: i32 = inv_ammo.min(16).min(16 - ammo);
                        inv_ammo -= reloaded;
                        ammo += reloaded;
                        items[0].equip();
                    }
                }

                // Items
                if item != 0 && mq::is_key_pressed(mq::KeyCode::Key1) {
                    item = 0;
                    rc::equip_item(&mut items, "gun");
                }

                // Item use
                if mq::is_mouse_button_pressed(mq::MouseButton::Left) {
                    // Animation
                    match item {
                        0 => {
                            if ammo > 0 {
                                items[item].texswap(&shooting_gun, 0.1);
                                ammo -= 1;
                                self.audio.play_sound("shoot");

                                // Cast gun ray
                                let ins: rc::Intersection = rc::cast_ray(&map, ents.ents.iter(), &['d'], cam);
                                match ins.itype {
                                    rc::IntersectionType::Entity { index, .. } => {
                                        // ents.remove(index);
                                        ents.death_timers[index] = Some(mq::get_time());
                                        ents.ents[index].texture = 'd';
                                        self.audio.play_sound("death");
                                    }
                                    _ => (),
                                }
                            } else {
                                self.audio.play_sound("dry");
                            }
                        },
                        _ => (),
                    }
                }

                if mq::is_mouse_button_pressed(mq::MouseButton::Right) {
                    grappling = true;
                    grapple_target = cam.along(rc::cast_ray(&map, ents.ents.iter(), &['e', 'd'], cam).distance);
                    self.audio.play_sound("grapple");
                }

                // Entity spawning
                let rng: i32 = mq::rand::gen_range(0, 100);
                if rng == 1 && ents.ents.len() < MAX_ENTS {
                    let pos: Vec2 = random_spot(&map);
                    ents.push(
                        rc::Entity::new(pos, 'e', (20., 30.)),
                        mq::rand::gen_range(1., 4.)
                    );
                }

                if rng == 2 && ammo_ents.len() < MAX_AMMO {
                    ammo_ents.push(rc::Entity::new(random_spot(&map), 'a', (20., 25.)));
                }

                if nut.is_empty() {
                    nut.push(rc::Entity::new(random_spot(&map), 'n', (20., 20.)));
                }

                // Ammo collect
                for (i, ent) in ammo_ents.iter().enumerate() {
                    if cam.orig.distance(ent.pos) < 20. {
                        // Can't remove multiple ents in a singe loop, just get the rest next frame
                        ammo_ents.remove(i);
                        inv_ammo += 32;
                        self.audio.play_sound("ammo");
                        break;
                    }
                }

                // Nuts collect
                if cam.orig.distance(nut[0].pos) < 20. {
                    nut.clear();
                    nuts_collected += 1;
                }

                // Remove dead entities
                for (i, death) in ents.death_timers.iter().enumerate() {
                    if let Some(death) = death {
                        if mq::get_time() - *death > 1. {
                            ents.remove(i);
                            break;
                        }
                    }
                }

                // Move entities
                for (ent, (speed, dead)) in ents.ents.iter_mut()
                                                     .zip(ents.speeds.iter()
                                                     .zip(ents.death_timers.iter()))
                {
                    if dead.is_some() {
                        continue;
                    }

                    let diff: Vec2 = cam.orig - ent.pos;
                    let theta: f32 = f32::atan2(diff.y, diff.x) + mq::rand::gen_range(-1.5, 1.5);
                    let dir: Vec2 = Vec2::new(theta.cos(), theta.sin());

                    ent.pos = rc::util::move_towards_collidable(&map, ent.pos, ent.pos + dir, *speed);
                }

                // Entities damage
                for (ent, death) in ents.ents.iter().zip(ents.death_timers.iter()) {
                    if death.is_none() && mq::get_time() - last_hurt >= 1. && cam.orig.distance(ent.pos) < 20. {
                        health -= 1;
                        last_hurt = mq::get_time();
                    }
                }
            } else {
                // Restart
                if mq::is_key_pressed(mq::KeyCode::Q) {
                    return;
                }
            }

            mq::clear_background(mq::BLACK);
            out_img.bytes.fill(0);
            rc::render(&map, ents.ents.iter().chain(nut.iter()).chain(ammo_ents.iter()), cam, rc::Fog::None, &mut out_img);
            out_tex.update(&out_img);
            let topleft: (f32, f32) = rc::scr_topleft();
            let shake: (f32, f32) = if mq::get_time() - shake_begin < 0.1 {
                (mq::rand::gen_range(-10., 10.), mq::rand::gen_range(-10., 10.))
            } else {
                (0., 0.)
            };
            mq::draw_texture(&out_tex, topleft.0 + shake.0, topleft.1 + shake.1, mq::WHITE);
            rc::render_item(&mut items);

            let cx: f32 = rc::scrw() as f32 / 2.;
            let cy: f32 = rc::scrh() as f32 / 2.;
            mq::draw_line(cx, cy - 10., cx, cy + 10., 2., mq::WHITE);
            mq::draw_line(cx - 10., cy, cx + 10., cy, 2., mq::WHITE);

            mq::draw_text(format!("LOADED:    {}", ammo).as_str(), 10., rc::scrh() as f32 - 40., 24., mq::WHITE);
            mq::draw_text(format!("INVENTORY: {}", inv_ammo).as_str(), 10., rc::scrh() as f32 - 20., 24., mq::WHITE);

            mq::draw_text(format!("HEALTH: {}", health).as_str(), 10., 20., 24., mq::WHITE);
            mq::draw_text(format!("NUTS:   {}", nuts_collected).as_str(), 10., 40., 24., mq::WHITE);

            if health == 0 || mq::get_time() - last_hurt < 1. {
                mq::draw_rectangle(0., 0., 800., 800., mq::Color::new(1., 0., 0., (1. - (mq::get_time() - last_hurt)) as f32 * 0.5));
            }

            if health == 0 || nuts_collected == NUTS_GOAL {
                mq::draw_rectangle(0., 0., 800., 800., mq::Color::new(0., 0., 0., 0.5));
            }

            if health == 0 {
                let text: &str = "Press [q] to restart";
                let measure = mq::measure_text(text, None, 24, 1.);
                mq::draw_text(text, rc::scrw() as f32 / 2. - measure.width / 2., rc::scrh() as f32 / 2. - measure.height / 2., 24., mq::WHITE);
            } else if nuts_collected == NUTS_GOAL {
                let text: &str = "All nuts were successfully collected. Press [q] to restart";
                let measure = mq::measure_text(text, None, 24, 1.);
                mq::draw_text(text, rc::scrw() as f32 / 2. - measure.width / 2., rc::scrh() as f32 / 2. - measure.height / 2., 24., mq::WHITE);
            }

            mq::next_frame().await;
        }
    }
}
