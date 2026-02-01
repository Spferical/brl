use bevy::{
    asset::RenderAssetUsages,
    image::{CompressedImageFormats, ImageSampler, ImageType},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use bevy_egui::{EguiContexts, EguiTextureHandle, egui};

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

    pub(crate) fn get_urizen_egui_image(
        &'_ self,
        contexts: &mut EguiContexts,
        atlas_assets: &Assets<TextureAtlasLayout>,
        index: usize,
    ) -> egui::Image<'_> {
        get_egui_image_from_sprite(contexts, atlas_assets, &self.get_urizen_sprite(index))
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

pub(crate) fn get_egui_image_from_sprite(
    contexts: &mut EguiContexts,
    atlas_assets: &Assets<TextureAtlasLayout>,
    sprite: &Sprite,
) -> egui::Image<'static> {
    let Some(ref texture_atlas) = sprite.texture_atlas else {
        panic!("get_egui_image_from_sprite only supports sprites with atlases")
    };
    let layout = atlas_assets.get(texture_atlas.layout.id()).unwrap();
    let rect: URect = layout.textures[texture_atlas.index];
    let rect = egui::Rect::from_min_max(
        egui::pos2(rect.min.x as f32, rect.min.y as f32),
        egui::pos2(rect.max.x as f32, rect.max.y as f32),
    );
    let uv = egui::Rect::from_min_max(
        egui::pos2(
            rect.min.x as f32 / layout.size.x as f32,
            rect.min.y as f32 / layout.size.y as f32,
        ),
        egui::pos2(
            // need to subtract one pixel or else it'll overlap the tiles to the right/below.
            (rect.max.x as f32 - 1f32) / layout.size.x as f32,
            (rect.max.y as f32 - 1f32) / layout.size.y as f32,
        ),
    );
    let texture_id = contexts.add_image(EguiTextureHandle::Weak(sprite.image.id()));
    let sized_texture = egui::load::SizedTexture::new(texture_id, rect.size());
    egui::Image::new(sized_texture).uv(uv)
}
