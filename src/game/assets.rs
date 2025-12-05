use bevy::{
    asset::RenderAssetUsages,
    image::{CompressedImageFormats, ImageSampler, ImageType},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

use crate::game::map::{TILE_HEIGHT, TILE_WIDTH};

#[derive(Resource, Asset, Clone, Reflect)]
#[reflect(Resource)]
pub struct WorldAssets {
    urizen: Handle<Image>,
    urizen_mask: Handle<Image>,
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

    pub(crate) fn get_urizen_sprite_mask(&self) -> Handle<Image> {
        self.urizen_mask.clone()
    }

    pub(crate) fn get_urizen_layout(&self) -> Handle<TextureAtlasLayout> {
        self.urizen_layout.clone()
    }
}

impl FromWorld for WorldAssets {
    fn from_world(world: &mut World) -> Self {
        let bytes = include_bytes!("../../assets/urizen_onebit_tileset__v2d0.png");
        let image = Image::from_buffer(
            bytes,
            ImageType::Extension("png"),
            CompressedImageFormats::NONE,
            true,
            ImageSampler::Default,
            RenderAssetUsages::default(),
        )
        .unwrap();

        let data = image.data.as_ref().expect("Image data should be present");
        let mut mask_data = Vec::with_capacity(data.len());
        for chunk in data.chunks(4) {
            let alpha = chunk[3];
            let val = if alpha > 0 { 255 } else { 0 };
            mask_data.extend_from_slice(&[val, val, val, val]);
        }

        let mask_image = Image::new(
            Extent3d {
                width: image.width(),
                height: image.height(),
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            mask_data,
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::default(),
        );

        let mut images = world.resource_mut::<Assets<Image>>();
        let urizen = images.add(image);
        let urizen_mask = images.add(mask_image);

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
            urizen_mask,
            urizen_layout,
        }
    }
}
