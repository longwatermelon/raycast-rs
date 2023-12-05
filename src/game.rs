use crate::audio::Audio;
use raycast::prelude as rc;
use rc::prelude::{macroquad, glam};
use macroquad::prelude as mq;
use glam::{Vec2, IVec2};
use std::collections::HashMap;

const MAX_ENTS: usize = 30;
const MAX_AMMO: usize = 3;
const NUTS_GOAL: i32 = 5;

struct Entities {
    ents: Vec<rc::Entity>,
    speeds: Vec<f32>,
    death_timers: Vec<Option<f64>>,
    velocities: Vec<Vec2>,
}

impl Entities {
    fn new() -> Self {
        Self {
            ents: Vec::new(),
            speeds: Vec::new(),
            death_timers: Vec::new(),
            velocities: Vec::new(),
        }
    }

    fn push(&mut self, ent: rc::Entity, speed: f32) {
        self.ents.push(ent);
        self.speeds.push(speed);
        self.death_timers.push(None);
        self.velocities.push(Vec2::ZERO);
    }

    fn remove(&mut self, index: usize) {
        self.ents.remove(index);
        self.speeds.remove(index);
        self.death_timers.remove(index);
        self.velocities.remove(index);
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
        textures.insert('E', mq::Image::from_file_with_format(include_bytes!("res/shrek-1.png"), Some(mq::ImageFormat::Png)).unwrap());
        textures.insert('D', mq::Image::from_file_with_format(include_bytes!("res/shrek-2.png"), Some(mq::ImageFormat::Png)).unwrap());
        textures.insert('d', mq::Image::from_file_with_format(include_bytes!("res/shrek_dead_gun.png"), Some(mq::ImageFormat::Png)).unwrap());
        textures.insert('n', mq::Image::from_file_with_format(include_bytes!("res/deez.png"), Some(mq::ImageFormat::Png)).unwrap());
        textures.insert('a', mq::Image::from_file_with_format(include_bytes!("res/ammo.png"), Some(mq::ImageFormat::Png)).unwrap());
        textures.insert('m', mq::Image::from_file_with_format(include_bytes!("res/mg-ammo.png"), Some(mq::ImageFormat::Png)).unwrap());
        textures.insert('x', mq::Image::from_file_with_format(include_bytes!("res/shrek-halved.png"), Some(mq::ImageFormat::Png)).unwrap());
        let mut map: rc::Map = rc::Map::from_bytes(include_bytes!("res/map"), textures);
        map.floor_tex(rc::Surface::Color(mq::DARKGRAY.into()));
        map.ceil_tex(rc::Surface::Color(mq::GRAY.into()));

        let mut ents: Entities = Entities::new();
        let mut nut: Vec<rc::Entity> = Vec::new();
        let mut ammo_ents: Vec<rc::Entity> = Vec::new();

        let mut items: Vec<rc::Item> = vec![
            rc::Item::new("knife", include_bytes!("res/knife.png")),
            rc::Item::new("mg", include_bytes!("res/machine-gun.png")),
            rc::Item::new("gun", include_bytes!("res/gun.png")),
        ];
        let mut item: usize = 0;
        rc::equip_item(&mut items, "knife");

        let shooting_gun: mq::Texture2D = mq::Texture2D::from_file_with_format(include_bytes!("res/gun-shoot.png"), Some(mq::ImageFormat::Png));
        let shooting_mg: mq::Texture2D = mq::Texture2D::from_file_with_format(include_bytes!("res/machine-gun-shoot.png"), Some(mq::ImageFormat::Png));

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
        let mut mg_ammo: i32 = 50;
        let mut inv_mg_ammo: i32 = 100;
        let mut reload_start: Option<f64> = None;

        let mut health: i32 = 5;
        let mut last_hurt: f64 = -100.;

        let mut nuts_collected: i32 = 0;

        let mut shake_begin: f64 = -100.;
        let mut mg_shake_begin: f64 = -100.;

        let mut wallh: f32;

        let mut mg_last_shot: f64 = -100.;

        let mut last_jab: f64 = -100.;

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
                    if cam.orig.distance(grapple_target) < if item == 0 { 40. } else { 20. } {
                        grappling = false;
                        self.audio.play_sound("impact");
                        shake_begin = mq::get_time();
                    } else {
                        cam.orig = rc::util::move_towards_collidable(&map, cam.orig, grapple_target, if item == 0 { 16. } else if item == 2 { 10. } else { 8. });
                    }
                } else {
                    rc::util::fps_camera_controls(&map, &mut cam, if item == 0 { 4. } else if item == 2 { 3. } else { 2. });
                }
                rc::util::fps_camera_rotation(&mut cam, &mut prev_mpos, 0.5);

                // Misc keys
                if (item == 2 || item == 1) && mq::is_key_pressed(mq::KeyCode::R) {
                    reload_start = Some(mq::get_time());
                    items[item].unequip();
                    self.audio.play_sound("reload");
                }

                // Reloading
                if let Some(start) = reload_start {
                    if mq::get_time() - start > 2. {
                        reload_start = None;
                        let n: i32 = if item == 2 { 16 } else { 50 };
                        let reloaded: i32 = (if item == 2 { inv_ammo } else { inv_mg_ammo }).min(n).min(n - if item == 2 { ammo } else { mg_ammo });

                        if item == 2 {
                            inv_ammo -= reloaded;
                            ammo += reloaded;
                        } else {
                            inv_mg_ammo -= reloaded;
                            mg_ammo += reloaded;
                        }
                        items[item].equip();
                    }
                }

                // Items
                if item != 2 && mq::is_key_pressed(mq::KeyCode::Key3) {
                    item = 2;
                    rc::equip_item(&mut items, "gun");
                }

                if item != 1 && mq::is_key_pressed(mq::KeyCode::Key2) {
                    item = 1;
                    rc::equip_item(&mut items, "mg");
                }

                if item != 0 && mq::is_key_pressed(mq::KeyCode::Key1) {
                    item = 0;
                    rc::equip_item(&mut items, "knife");
                }

                // Item use
                if mq::is_mouse_button_pressed(mq::MouseButton::Left) {
                    // Animation
                    match item {
                        2 => {
                            if ammo > 0 {
                                mg_shake_begin = mq::get_time();
                                items[item].texswap(&shooting_gun, 0.1);
                                ammo -= 1;
                                self.audio.play_sound("shoot");

                                // Cast gun ray
                                let ins: rc::Intersection = rc::cast_ray(&map, ents.ents.iter(), &['d'], cam);
                                match ins.itype {
                                    rc::IntersectionType::Entity { index, .. } => {
                                        // ents.remove(index);
                                        ents.ents[index].texture = match ents.ents[index].texture {
                                            'e' => 'E',
                                            'E' => 'D',
                                            'D' => 'd',
                                            _ => 'd',
                                        };

                                        self.audio.play_sound("damage");
                                        if ents.ents[index].texture == 'd' {
                                            ents.death_timers[index] = Some(mq::get_time());
                                            self.audio.play_sound("death");
                                        }
                                    }
                                    _ => (),
                                }
                            } else {
                                self.audio.play_sound("dry");
                            }
                        }
                        0 => {
                            items[0].jab(if grappling { Vec2::new(-50., -50.) } else { Vec2::new(-100., 100.) }, 0.05);
                            last_jab = mq::get_time();

                        }
                        _ => (),
                    }
                }

                if mq::is_mouse_button_down(mq::MouseButton::Left) {
                    match item {
                        1 => {
                            if mq::get_time() - mg_last_shot > 0.1 {
                                mg_last_shot = mq::get_time();
                                mg_shake_begin = mq::get_time();
                                if mg_ammo > 0 {
                                    items[item].texswap(&shooting_mg, 0.1);
                                    mg_ammo -= 1;
                                    self.audio.play_sound("shoot");

                                    // Cast gun ray
                                    let ins: rc::Intersection = rc::cast_ray(&map, ents.ents.iter(), &['d'], cam);
                                    match ins.itype {
                                        rc::IntersectionType::Entity { index, .. } => {
                                            // ents.remove(index);
                                            ents.ents[index].texture = match ents.ents[index].texture {
                                                'e' => 'E',
                                                'E' => 'D',
                                                'D' => 'd',
                                                _ => 'd',
                                            };

                                            self.audio.play_sound("damage");
                                            if ents.ents[index].texture == 'd' {
                                                ents.death_timers[index] = Some(mq::get_time());
                                                self.audio.play_sound("death");
                                            }
                                        }
                                        _ => (),
                                    }
                                } else {
                                    self.audio.play_sound("dry");
                                }
                            }
                        }
                        _ => (),
                    }
                }

                if mq::get_time() - last_jab < 0.1 {
                    let mut hit_ents: bool = false;
                    for (ent, (death, vel)) in ents.ents.iter_mut().zip(ents.death_timers.iter_mut().zip(ents.velocities.iter_mut())) {
                        if ent.texture == 'x' || vel.x.abs() > 0.001 || vel.y.abs() > 0.001 {
                            continue;
                        }

                        if ent.pos.distance(cam.orig) < 30. && (ent.pos - cam.orig).normalize().dot(cam.dir()) > 0.2 {
                            if grappling {
                                *vel = (grapple_target - cam.orig).normalize();
                                hit_ents = true;
                            } else {
                                ent.texture = 'x';
                                *death = Some(mq::get_time());
                                self.audio.play_sound("damage");
                            }
                        }
                    }

                    if hit_ents {
                        grappling = false;
                    }
                }

                if mq::is_mouse_button_pressed(mq::MouseButton::Right) {
                    grappling = true;
                    grapple_target = cam.along(rc::cast_ray(&map, ents.ents.iter(), &['e', 'd', 'x', 'a', 'm'], cam).distance);
                    self.audio.play_sound("grapple");
                }

                // Entity spawning
                let rng: i32 = mq::rand::gen_range(0, 100);
                if rng < 3 && ents.ents.len() < MAX_ENTS {
                    let pos: Vec2 = random_spot(&map);
                    ents.push(
                        rc::Entity::new(pos, 'e', (20., 30.)),
                        mq::rand::gen_range(1., 4.)
                    );
                }

                if (rng == 2 || rng == 3) && ammo_ents.len() < MAX_AMMO {
                    ammo_ents.push(rc::Entity::new(random_spot(&map), if rng == 2 { 'a' } else { 'm' }, (20., 25.)));
                }

                if nut.is_empty() {
                    nut.push(rc::Entity::new(random_spot(&map), 'n', (20., 20.)));
                }

                // Ammo collect
                for (i, ent) in ammo_ents.iter().enumerate() {
                    if cam.orig.distance(ent.pos) < 20. {
                        // Can't remove multiple ents in a singe loop, just get the rest next frame
                        if ent.texture == 'a' {
                            inv_ammo += 32;
                        } else {
                            inv_mg_ammo += 100;
                        }
                        ammo_ents.remove(i);
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
                for (ent, (speed, (dead, vel))) in ents.ents.iter_mut()
                                                     .zip(ents.speeds.iter()
                                                     .zip(ents.death_timers.iter_mut()
                                                     .zip(ents.velocities.iter())))
                {
                    if dead.is_some() {
                        continue;
                    }

                    if vel.x.abs() > 0.001 || vel.y.abs() > 0.001 {
                        let orig_pos: Vec2 = ent.pos;
                        ent.pos = rc::util::move_towards_collidable(&map, ent.pos, ent.pos + *vel, 16.);
                        if ent.pos.distance(orig_pos + *vel * 16.) > 5. {
                            *dead = Some(mq::get_time());
                            ent.texture = 'x';
                            self.audio.play_sound("impact");
                            self.audio.play_sound("death");
                            shake_begin = mq::get_time();
                        }
                    } else {
                        let diff: Vec2 = cam.orig - ent.pos;
                        let theta: f32 = f32::atan2(diff.y, diff.x) + mq::rand::gen_range(-1.5, 1.5);
                        let dir: Vec2 = Vec2::new(theta.cos(), theta.sin());

                        ent.pos = rc::util::move_towards_collidable(&map, ent.pos, ent.pos + dir, *speed);
                    }
                }

                // Entities damage
                for (ent, death) in ents.ents.iter().zip(ents.death_timers.iter()) {
                    if death.is_none() && mq::get_time() - last_hurt >= 1. && cam.orig.distance(ent.pos) < 5. {
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
            rc::render(&map, ents.ents.iter().chain(nut.iter()).chain(ammo_ents.iter()), cam, rc::Fog::None, &|| 0., &mut out_img);
            out_tex.update(&out_img);
            let topleft: (f32, f32) = rc::scr_topleft();
            let shake: (f32, f32) = if mq::get_time() - shake_begin < 0.1 {
                (mq::rand::gen_range(-10., 10.), mq::rand::gen_range(-10., 10.))
            } else {
                (0., 0.)
            };

            let mg_shake: (f32, f32) = if mq::get_time() - mg_shake_begin < 0.05 {
                (mq::rand::gen_range(-5., 5.), mq::rand::gen_range(-5., 5.))
            } else {
                (0., 0.)
            };
            mq::draw_texture(&out_tex, topleft.0 + shake.0 + mg_shake.0, topleft.1 + shake.1 + mg_shake.1, mq::WHITE);
            rc::render_item(&mut items);

            let cx: f32 = rc::scrw() as f32 / 2.;
            let cy: f32 = rc::scrh() as f32 / 2.;

            if item != 0 {
                mq::draw_line(topleft.0 + cx, topleft.1 + cy - 10., topleft.0 + cx, topleft.1 + cy + 10., 2., mq::WHITE);
                mq::draw_line(topleft.0 + cx - 10., topleft.1 + cy, topleft.0 + cx + 10., topleft.1 + cy, 2., mq::WHITE);
                mq::draw_text(format!("LOADED:    {}", if item == 2 { ammo } else { mg_ammo }).as_str(), topleft.0 + 10., topleft.1 + rc::scrh() as f32 - 40., 24., mq::WHITE);
                mq::draw_text(format!("INVENTORY: {}", if item == 2 { inv_ammo } else { inv_mg_ammo }).as_str(), topleft.0 + 10., topleft.1 + rc::scrh() as f32 - 20., 24., mq::WHITE);
            }

            mq::draw_text(format!("HEALTH: {}", health).as_str(), topleft.0 + 10., topleft.1 + 20., 24., mq::WHITE);
            mq::draw_text(format!("NUTS:   {}", nuts_collected).as_str(), topleft.0 + 10., topleft.1 + 40., 24., mq::WHITE);

            mq::draw_text(format!("FPS {}", mq::get_fps()).as_str(), topleft.0 + rc::scrw() as f32 - 80., topleft.1 + 20., 24., mq::WHITE);

            if health == 0 || mq::get_time() - last_hurt < 1. {
                mq::draw_rectangle(topleft.0, topleft.1, rc::scrw() as f32, rc::scrh() as f32, mq::Color::new(1., 0., 0., (1. - (mq::get_time() - last_hurt)) as f32 * 0.5));
            }

            if health == 0 || nuts_collected == NUTS_GOAL {
                mq::draw_rectangle(topleft.0, topleft.1, rc::scrw() as f32, rc::scrh() as f32, mq::Color::new(0., 0., 0., 0.5));
            }

            if health == 0 {
                let text: &str = "Press [q] to restart";
                let measure = mq::measure_text(text, None, 24, 1.);
                mq::draw_text(text, topleft.0 + rc::scrw() as f32 / 2. - measure.width / 2., topleft.1 + rc::scrh() as f32 / 2. - measure.height / 2., 24., mq::WHITE);
            } else if nuts_collected == NUTS_GOAL {
                let text: &str = "All nuts were successfully collected. Press [q] to restart";
                let measure = mq::measure_text(text, None, 24, 1.);
                mq::draw_text(text, topleft.0 + rc::scrw() as f32 / 2. - measure.width / 2., topleft.1 +  rc::scrh() as f32 / 2. - measure.height / 2., 24., mq::WHITE);
            }

            mq::next_frame().await;
        }
    }
}
