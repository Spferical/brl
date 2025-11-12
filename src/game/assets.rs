use bevy::prelude::*;

use crate::game::map::{TILE_HEIGHT, TILE_WIDTH};
#[derive(Resource, Asset, Clone, Reflect)]
#[reflect(Resource)]
pub struct WorldAssets {
    #[dependency]
    urizen: Handle<Image>,
    urizen_layout: Handle<TextureAtlasLayout>,
}

impl WorldAssets {
    pub(crate) fn get_urizen_sprite(&self, index: usize) -> Sprite {
        let mut player_sprite = Sprite::from_atlas_image(
            self.urizen.clone(),
            TextureAtlas {
                layout: self.urizen_layout.clone(),
                index,
            },
        );
        player_sprite.custom_size = Some(Vec2::new(TILE_WIDTH, TILE_HEIGHT));
        player_sprite
    }
}

impl FromWorld for WorldAssets {
    fn from_world(world: &mut World) -> Self {
        let assets = world.resource::<AssetServer>();
        let urizen = assets.load("urizen_onebit_tileset__v2d0.png");
        let mut tals = world.resource_mut::<Assets<TextureAtlasLayout>>();
        let urizen_layout = tals.add(TextureAtlasLayout::from_grid(
            UVec2::splat(12),
            206,
            50,
            Some(UVec2::splat(1)),
            Some(UVec2::splat(1)),
        ));
        Self {
            urizen,
            urizen_layout,
        }
    }
}
