use raycast::prelude as rc;
use rc::prelude::{macroquad, glam};
use macroquad::prelude as mq;
use glam::Vec2;
use std::collections::HashMap;

const MAX_ENTS: usize = 15;

#[macroquad::main(window_conf)]
async fn main() {
    rc::util::set_scrw_scrh(800, 800);

    let mut textures: HashMap<char, mq::Image> = HashMap::new();
    textures.insert('0', mq::Image::from_file_with_format(include_bytes!("res/wall.png"), Some(mq::ImageFormat::Png)).unwrap());
    textures.insert('e', mq::Image::from_file_with_format(include_bytes!("res/shrek.png"), Some(mq::ImageFormat::Png)).unwrap());
    let mut map: rc::Map = rc::Map::from_bytes(include_bytes!("res/map"), textures);
    map.floor_tex(rc::Surface::Color(mq::BEIGE.into()));
    map.ceil_tex(rc::Surface::Color(mq::GRAY.into()));

    let mut entities: Vec<rc::Entity> = Vec::new();
    let mut ent_speeds: Vec<f32> = Vec::new();

    let mut items: Vec<rc::Item> = vec![
        rc::Item::new("gun", include_bytes!("res/gun.png")),
    ];
    let mut item: usize = 0;

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

    loop {
        if mq::is_key_pressed(mq::KeyCode::Tab) {
            grabbed = !grabbed;
            mq::set_cursor_grab(grabbed);
            mq::show_mouse(!grabbed);
        }

        // Movement
        raycast::util::fps_camera_controls(&map, &mut cam, 2.);
        raycast::util::fps_camera_rotation(&mut cam, &mut prev_mpos, 1.);

        // Items
        if mq::is_key_pressed(mq::KeyCode::Key1) {
            item = 0;
            rc::equip_item(&mut items, "gun");
        }

        // Item use
        if mq::is_mouse_button_pressed(mq::MouseButton::Left) {
            // Animation
            match item {
                0 => items[item].texswap(&shooting_gun, 0.1),
                _ => (),
            }

            // Raycast
            let ins: rc::Intersection = rc::cast_ray(&map, entities.iter(), &[], cam);
            match ins.itype {
                rc::IntersectionType::Entity { index, .. } => println!("Hit entity {}", index),
                _ => (),
            }
        }

        // Spawn entities
        let rng: i32 = mq::rand::gen_range(0, 100);
        if rng == 1 && entities.len() < MAX_ENTS {
            entities.push(rc::Entity::new(Vec2::new(100., 200.), 'e', (20., 30.)));
            ent_speeds.push(mq::rand::gen_range(1., 4.));
        }

        // Move entities
        for (ent, speed) in entities.iter_mut().zip(ent_speeds.iter()) {
            let diff: Vec2 = cam.orig - ent.pos;
            let theta: f32 = f32::atan2(diff.y, diff.x) + mq::rand::gen_range(-1.5, 1.5);
            let dir: Vec2 = Vec2::new(theta.cos(), theta.sin());

            ent.pos = rc::util::move_towards_collidable(&map, ent.pos, ent.pos + dir, *speed);
        }

        mq::clear_background(mq::BLACK);
        out_img.bytes.fill(0);
        rc::render(&map, entities.iter(), cam, rc::Fog::None, &mut out_img);
        out_tex.update(&out_img);
        mq::draw_texture(&out_tex, 0., 0., mq::WHITE);
        rc::render_item(&mut items);
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
