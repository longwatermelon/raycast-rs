mod audio;

use audio::Audio;
use raycast::prelude as rc;
use rc::prelude::{macroquad, glam};
use macroquad::prelude as mq;
use glam::{Vec2, IVec2};
use std::collections::HashMap;

const MAX_ENTS: usize = 15;

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

#[macroquad::main(window_conf)]
async fn main() {
    rc::util::set_scrw_scrh(800, 800);

    let mut textures: HashMap<char, mq::Image> = HashMap::new();
    textures.insert('0', mq::Image::from_file_with_format(include_bytes!("res/wall.png"), Some(mq::ImageFormat::Png)).unwrap());
    textures.insert('e', mq::Image::from_file_with_format(include_bytes!("res/shrek.png"), Some(mq::ImageFormat::Png)).unwrap());
    textures.insert('d', mq::Image::from_file_with_format(include_bytes!("res/shrek_dead_gun.png"), Some(mq::ImageFormat::Png)).unwrap());
    let mut map: rc::Map = rc::Map::from_bytes(include_bytes!("res/map"), textures);
    map.floor_tex(rc::Surface::Color(mq::BEIGE.into()));
    map.ceil_tex(rc::Surface::Color(mq::GRAY.into()));

    let mut ents: Entities = Entities::new();

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

    let audio: Audio = Audio::new().await;
    audio.loop_sound("music");

    let mut grappling: bool = false;
    let mut grapple_target: Vec2 = Vec2::default();

    loop {
        if mq::is_key_pressed(mq::KeyCode::Escape) {
            grabbed = !grabbed;
            mq::set_cursor_grab(grabbed);
            mq::show_mouse(!grabbed);
        }

        // Movement
        if grappling {
            cam.orig = rc::util::move_towards_collidable(&map, cam.orig, grapple_target, 8.);
            if cam.orig.distance(grapple_target) < 20. {
                grappling = false;
                audio.play_sound("impact");
            }
        } else {
            rc::util::fps_camera_controls(&map, &mut cam, 2.);
        }
        rc::util::fps_camera_rotation(&mut cam, &mut prev_mpos, 1.);

        // Items
        if mq::is_key_pressed(mq::KeyCode::Key1) {
            item = 0;
            rc::equip_item(&mut items, "gun");
        }

        // Item use
        if mq::is_mouse_button_pressed(mq::MouseButton::Left) {
            // Animation
            match item {
                0 => {
                    items[item].texswap(&shooting_gun, 0.1);
                    audio.play_sound("shoot");
                },
                _ => (),
            }

            // Raycast
            let ins: rc::Intersection = rc::cast_ray(&map, ents.ents.iter(), &['d'], cam);
            match ins.itype {
                rc::IntersectionType::Entity { index, .. } => {
                    // ents.remove(index);
                    ents.death_timers[index] = Some(mq::get_time());
                    ents.ents[index].texture = 'd';
                    audio.play_sound("death");
                }
                _ => (),
            }
        }

        if mq::is_mouse_button_pressed(mq::MouseButton::Right) {
            grappling = true;
            grapple_target = cam.along(rc::cast_ray(&map, ents.ents.iter(), &['e', 'd'], cam).distance);
            audio.play_sound("grapple");
        }

        // Spawn entities
        let rng: i32 = mq::rand::gen_range(0, 100);
        if rng == 1 && ents.ents.len() < MAX_ENTS {
            let pos: Vec2 = random_spot(&map);
            ents.push(
                rc::Entity::new(pos, 'e', (20., 30.)),
                mq::rand::gen_range(1., 4.)
            );
        }

        // Remove dead entities
        for (i, death) in ents.death_timers.iter().enumerate() {
            if let Some(death) = death {
                if mq::get_time() - *death > 1. {
                    // Can't remove multiple ents in a singe loop, just get the rest next frame
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

        mq::clear_background(mq::BLACK);
        out_img.bytes.fill(0);
        rc::render(&map, ents.ents.iter(), cam, rc::Fog::None, &mut out_img);
        out_tex.update(&out_img);
        mq::draw_texture(&out_tex, 0., 0., mq::WHITE);
        rc::render_item(&mut items);

        let cx: f32 = mq::screen_width() / 2.;
        let cy: f32 = mq::screen_height() / 2.;
        mq::draw_line(cx, cy - 10., cx, cy + 10., 2., mq::WHITE);
        mq::draw_line(cx - 10., cy, cx + 10., cy, 2., mq::WHITE);

        mq::next_frame().await;
    }
}

fn window_conf() -> mq::Conf {
    mq::Conf {
        window_title: String::from("raycast"),
        window_width: 800,
        window_height: 800,
        window_resizable: false,
        ..Default::default()
    }
}
