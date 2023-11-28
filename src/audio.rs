use raycast::prelude::macroquad as mq;
use mq::audio::{self, Sound, PlaySoundParams};
use std::collections::HashMap;

pub struct Audio {
    sounds: HashMap<&'static str, Sound>,
}

impl Audio {
    pub async fn new() -> Self {
        let mut sounds: HashMap<&'static str, Sound> = HashMap::new();
        sounds.insert("music", audio::load_sound_from_bytes(include_bytes!("res/shreksophone.wav")).await.unwrap());
        sounds.insert("shoot", audio::load_sound_from_bytes(include_bytes!("res/gunshot.wav")).await.unwrap());
        sounds.insert("death", audio::load_sound_from_bytes(include_bytes!("res/death.wav")).await.unwrap());
        sounds.insert("ammo", audio::load_sound_from_bytes(include_bytes!("res/ammo.wav")).await.unwrap());
        sounds.insert("grapple", audio::load_sound_from_bytes(include_bytes!("res/grapple.wav")).await.unwrap());
        sounds.insert("impact", audio::load_sound_from_bytes(include_bytes!("res/impact.wav")).await.unwrap());
        sounds.insert("reload", audio::load_sound_from_bytes(include_bytes!("res/reload.wav")).await.unwrap());

        Self { sounds }
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

    pub fn loop_sound(&self, name: &str) {
        audio::play_sound(
            self.sounds.get(name).unwrap(),
            PlaySoundParams {
                looped: true,
                volume: 1.,
            }
        );
    }
}
