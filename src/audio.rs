use bevy::{audio::Volume, prelude::*};

#[cfg(target_arch = "wasm32")]
use web_sys::{Blob, BlobPropertyBag, HtmlAudioElement, Url};

#[cfg(target_arch = "wasm32")]
#[derive(Component)]
struct WebAudioSink {
    element: HtmlAudioElement,
    object_url: String,
}

#[cfg(target_arch = "wasm32")]
impl WebAudioSink {
    fn set_volume(&mut self, volume: Volume) {
        self.element.set_volume(volume.to_linear() as f64);
    }

    fn set_speed(&mut self, speed: f32) {
        let _ = self.element.set_playback_rate(speed as f64);
    }
}

#[cfg(target_arch = "wasm32")]
impl Drop for WebAudioSink {
    fn drop(&mut self) {
        let _ = self.element.pause();
        let _ = Url::revoke_object_url(&self.object_url);
        if let Some(parent) = self.element.parent_node() {
            let _ = parent.remove_child(&self.element);
        }
    }
}

#[cfg(target_arch = "wasm32")]
unsafe impl Send for WebAudioSink {}
#[cfg(target_arch = "wasm32")]
unsafe impl Sync for WebAudioSink {}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<MemeSoundTimer>();
    app.add_systems(
        Update,
        (
            #[cfg(target_arch = "wasm32")]
            (spawn_web_audio, despawn_finished_web_audio),
            apply_global_volume.run_if(resource_changed::<GlobalVolume>),
            start_music.run_if(
                resource_exists::<crate::game::assets::WorldAssets>
                    .and(in_state(crate::screens::Screen::Gameplay)),
            ),
            (play_meme_sounds, update_music_speed)
                .run_if(in_state(crate::screens::Screen::Gameplay)),
            (fade_out_music, stop_music_on_fade_out)
                .run_if(in_state(crate::screens::Screen::GameOver)),
        ),
    );
    app.add_systems(OnEnter(crate::screens::Screen::GameOver), start_fade_out);
}

#[derive(Component)]
struct MusicFadeOut {
    initial_volume: f32,
    timer: Timer,
}

fn start_fade_out(
    mut commands: Commands,
    music_query: Query<(Entity, &PlaybackSettings), With<Music>>,
) {
    for (entity, playback) in &music_query {
        commands.entity(entity).insert(MusicFadeOut {
            initial_volume: playback.volume.to_linear(),
            timer: Timer::from_seconds(1.0, TimerMode::Once),
        });
    }
}

fn fade_out_music(
    time: Res<Time>,
    global_volume: Res<GlobalVolume>,
    #[cfg(not(target_arch = "wasm32"))] mut music_query: Query<(&mut MusicFadeOut, &mut AudioSink)>,
    #[cfg(target_arch = "wasm32")] mut music_query: Query<(&mut MusicFadeOut, &mut WebAudioSink)>,
) {
    for (mut fade, mut sink) in &mut music_query {
        fade.timer.tick(time.delta());
        let t = 1.0 - fade.timer.fraction();
        sink.set_volume(global_volume.volume * Volume::Linear(fade.initial_volume * t));
    }
}

fn stop_music_on_fade_out(mut commands: Commands, music_query: Query<(Entity, &MusicFadeOut)>) {
    for (entity, fade) in &music_query {
        if fade.timer.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

#[derive(Resource, Default)]
struct MemeSoundTimer(Timer);

fn play_meme_sounds(
    mut commands: Commands,
    assets: Res<crate::game::assets::WorldAssets>,
    player: Single<&crate::game::Player>,
    mut timer: ResMut<MemeSoundTimer>,
    time: Res<Time>,
) {
    let player = player.into_inner();
    if player.brainrot < 80 {
        return;
    }

    timer.0.tick(time.delta());

    if timer.0.is_finished() || timer.0.duration().as_secs() == 0 {
        // Play a random meme sound at a lower fixed volume
        use rand::seq::IndexedRandom;
        let mut rng = rand::rng();
        if let Some(sound) = assets.meme_sounds.choose(&mut rng) {
            commands.spawn(sound_effect(sound.clone()));
        }

        // Set next timer duration based on brainrot
        // 80 brainrot: 30-60s
        // 100+ brainrot: 3-8s
        let t = ((player.brainrot - 80) as f32 / 20.0).clamp(0.0, 1.0);
        use rand::Rng;
        let mut rng = rand::rng();
        let next_duration = if player.brainrot >= 100 {
            rng.random_range(3.0..8.0)
        } else {
            // Linear interpolation between 80 and 100
            let min = 30.0 - (30.0 - 3.0) * t;
            let max = 60.0 - (60.0 - 8.0) * t;
            rng.random_range(min..max)
        };
        timer
            .0
            .set_duration(std::time::Duration::from_secs_f32(next_duration));
        timer.0.reset();
    }
}

fn update_music_speed(
    player: Single<&crate::game::Player>,
    #[cfg(not(target_arch = "wasm32"))] mut music_query: Query<&mut AudioSink, With<Music>>,
    #[cfg(target_arch = "wasm32")] mut music_query: Query<&mut WebAudioSink, With<Music>>,
) {
    let player = player.into_inner();
    let speed = if player.brainrot < 80 {
        1.0
    } else if player.brainrot >= 100 {
        1.2
    } else {
        // Linear between 80 and 100
        let t = (player.brainrot - 80) as f32 / 20.0;
        1.0 + t * 0.2
    };

    for mut sink in &mut music_query {
        sink.set_speed(speed);
    }
}

/// Start playing the music if it's not already playing.
fn start_music(
    mut commands: Commands,
    assets: Res<crate::game::assets::WorldAssets>,
    music_query: Query<Entity, With<Music>>,
) {
    if music_query.is_empty() {
        commands.spawn(music(assets.music.clone()));
    }
}

/// An organizational marker component that should be added to a spawned [`AudioPlayer`] if it's in the
/// general "music" category (e.g. global background music, soundtrack).
///
/// This can then be used to query for and operate on sounds in that category.
#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct Music;

#[cfg(target_arch = "wasm32")]
#[derive(Component)]
struct WebAudioHandle(Handle<AudioSource>);

/// A music audio instance.
#[allow(unused)]
pub fn music(handle: Handle<AudioSource>) -> impl Bundle {
    (
        #[cfg(not(target_arch = "wasm32"))]
        AudioPlayer(handle.clone()),
        #[cfg(target_arch = "wasm32")]
        WebAudioHandle(handle),
        PlaybackSettings {
            mode: bevy::audio::PlaybackMode::Loop,
            volume: bevy::audio::Volume::Linear(0.3),
            ..default()
        },
        Music,
    )
}

/// An organizational marker component that should be added to a spawned [`AudioPlayer`] if it's in the
/// general "sound effect" category (e.g. footsteps, the sound of a magic spell, a door opening).
///
/// This can then be used to query for and operate on sounds in that category.
#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct SoundEffect;

/// A sound effect audio instance.
#[allow(unused)]
pub fn sound_effect(handle: Handle<AudioSource>) -> impl Bundle {
    (
        #[cfg(not(target_arch = "wasm32"))]
        AudioPlayer(handle.clone()),
        #[cfg(target_arch = "wasm32")]
        WebAudioHandle(handle),
        PlaybackSettings {
            mode: bevy::audio::PlaybackMode::Despawn,
            volume: bevy::audio::Volume::Linear(0.6),
            ..default()
        },
        SoundEffect,
    )
}

/// [`GlobalVolume`] doesn't apply to already-running audio entities, so this system will update them.
fn apply_global_volume(
    global_volume: Res<GlobalVolume>,
    #[cfg(not(target_arch = "wasm32"))] mut audio_query: Query<
        (&PlaybackSettings, &mut AudioSink),
        Without<MusicFadeOut>,
    >,
    #[cfg(target_arch = "wasm32")] mut audio_query: Query<
        (&PlaybackSettings, &mut WebAudioSink),
        Without<MusicFadeOut>,
    >,
) {
    for (playback, mut sink) in &mut audio_query {
        sink.set_volume(global_volume.volume * playback.volume);
    }
}

#[cfg(target_arch = "wasm32")]
fn spawn_web_audio(
    mut commands: Commands,
    query: Query<
        (Entity, &WebAudioHandle, &PlaybackSettings),
        (Or<(With<Music>, With<SoundEffect>)>, Without<WebAudioSink>),
    >,
    assets: Res<Assets<AudioSource>>,
) {
    for (entity, handle, settings) in &query {
        if let Some(source) = assets.get(&handle.0) {
            let bytes = source.bytes.as_ref();
            let uint8_array = js_sys::Uint8Array::from(bytes);
            let blob_parts = js_sys::Array::new();
            blob_parts.push(&uint8_array);

            let blob_options = BlobPropertyBag::new();
            blob_options.set_type("audio/ogg");

            let blob =
                Blob::new_with_u8_array_sequence_and_options(&blob_parts, &blob_options).unwrap();
            let url = Url::create_object_url_with_blob(&blob).unwrap();

            let element = HtmlAudioElement::new_with_src(&url).unwrap();
            element.set_volume(settings.volume.to_linear() as f64);
            element.set_loop(matches!(settings.mode, bevy::audio::PlaybackMode::Loop));

            if let Some(window) = web_sys::window() {
                if let Some(document) = window.document() {
                    if let Some(body) = document.body() {
                        let _ = body.append_child(&element);
                    }
                }
            }

            let _ = element.play();

            commands.entity(entity).insert(WebAudioSink {
                element,
                object_url: url,
            });
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn despawn_finished_web_audio(
    mut commands: Commands,
    query: Query<(Entity, &WebAudioSink, &PlaybackSettings)>,
) {
    for (entity, sink, settings) in &query {
        if (matches!(settings.mode, bevy::audio::PlaybackMode::Despawn)
            || matches!(settings.mode, bevy::audio::PlaybackMode::Once))
            && sink.element.ended()
        {
            commands.entity(entity).despawn();
        }
    }
}
