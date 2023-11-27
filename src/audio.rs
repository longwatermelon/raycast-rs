use raycast::prelude::macroquad as mq;
use mq::audio::{self, Sound, PlaySoundParams};
use std::collections::HashMap;

pub struct Audio {
    sounds: HashMap<&'static str, Sound>,
}

impl Audio {
    pub async fn new() -> Self {
        let mut sounds: HashMap<&'static str, Sound> = HashMap::new();
        sounds.insert("shoot", audio::load_sound_from_bytes(include_bytes!("res/gunshot.wav")).await.unwrap());

        Self {
            sounds
        }
    }

    pub fn play_sound(&self, name: &str) {
        audio::play_sound(
            self.sounds.get(name).unwrap(),
            PlaySoundParams {
                looped: false,
                volume: 1.,
            }
        );
    }

//     pub fn stop_sound(&self, name: &str) {
//         audio::stop_sound(self.sounds.get(name).unwrap());
//     }
}
