mod audio;
mod game;

use game::Game;
use raycast::prelude::macroquad;
use macroquad::prelude as mq;

#[macroquad::main(window_conf)]
async fn main() {
    mq::rand::srand(mq::get_time() as u64);
    let game: Game = Game::new().await;

    loop {
        game.run().await;
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
