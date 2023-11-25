use raycast::prelude as rc;
use rc::prelude::{macroquad, glam};
use macroquad::prelude as mq;
use glam::Vec2;
use std::collections::HashMap;

#[macroquad::main(window_conf)]
async fn main() {
    rc::util::set_scrw_scrh(800, 800);

    let mut textures: HashMap<char, mq::Image> = HashMap::new();
    textures.insert('0', mq::Image::from_file_with_format(include_bytes!("res/wall.png"), Some(mq::ImageFormat::Png)).unwrap());
    let mut map: rc::Map = rc::Map::from_bytes(include_bytes!("res/map"), textures);
    map.floor_tex(rc::Surface::Color(mq::BEIGE.into()));
    map.ceil_tex(rc::Surface::Color(mq::GRAY.into()));

    let mut entities: Vec<rc::Entity> = Vec::new();

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

        raycast::util::fps_camera_controls(&map, &mut cam, 2.);
        raycast::util::fps_camera_rotation(&mut cam, &mut prev_mpos, 1.);

        mq::clear_background(mq::BLACK);
        out_img.bytes.fill(0);
        rc::render(&map, entities.iter(), cam, rc::Fog::None, &mut out_img);
        out_tex.update(&out_img);
        mq::draw_texture(&out_tex, 0., 0., mq::WHITE);
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
