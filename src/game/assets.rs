use bevy::{
    asset::RenderAssetUsages,
    image::{CompressedImageFormats, ImageSampler, ImageType},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use bevy_egui::{
    EguiContexts, EguiTextureHandle,
    egui::{self, Color32},
};
use geo::{ConvexHull, MultiPoint, Point};
use std::collections::HashMap;

use crate::game::map::{TILE_HEIGHT, TILE_WIDTH};

#[derive(Clone, Reflect)]
pub struct PhoneAppIcons {
    pub crawlr: Handle<Image>,
    pub dungeon_dash: Handle<Image>,
    pub underground_tv: Handle<Image>,
    pub cockatrice: Handle<Image>,
}

#[derive(Resource, Asset, Clone, Reflect)]
#[reflect(Resource)]
pub struct WorldAssets {
    pub font: Handle<Font>,
    pub comic_relief: Handle<Font>,
    pub comic_relief_bold: Handle<Font>,
    urizen: Handle<Image>,
    urizen_mask: Handle<Image>,
    urizen_layout: Handle<TextureAtlasLayout>,
    ffz_tileset: Handle<Image>,
    ffz_layout: Handle<TextureAtlasLayout>,
    pub valid_emote_indices: Option<Vec<usize>>,
    pub urizen_hulls: Vec<Vec<Vec2>>,
    pub char_hulls: HashMap<char, Vec<Vec2>>,
    solid_mask: Handle<Image>,
    pub phone: Handle<Image>,
    pub phone_app_icons: PhoneAppIcons,
    pub music: Handle<AudioSource>,
    pub meme_sounds: Vec<Handle<AudioSource>>,
    pub oof: Handle<AudioSource>,
    pub button_click: Handle<AudioSource>,
    pub button_hover: Handle<AudioSource>,
}

pub const FORBIDDEN_EMOTE_IDS: &[u32] = &[
    28, 41, 64, 75, 82, 128, 151, 175, 189, 208, 275, 276, 325, 333, 345, 361, 362, 368, 379, 386,
    492, 535, 564, 558, 559, 588, 607, 611, 619, 635, 643, 659, 660, 661, 662, 685, 687, 704, 716,
    717, 720, 722, 732, 751, 769, 790, 794, 808, 822, 835, 843, 847, 854, 860, 874, 913, 923, 935,
    938, 941, 948, 953, 958, 982, 985, 1002, 1011, 1022, 1023,
];

#[derive(Component, Clone, Copy, Reflect)]
pub struct BaseColor(pub Color);

pub type AsciiSprite = (Text2d, TextFont, TextColor, BaseColor);

impl WorldAssets {
    pub(crate) fn get_ascii_sprite(&self, c: char, color: Color) -> AsciiSprite {
        (
            Text2d::new(c.to_string()),
            TextFont {
                font: self.font.clone(),
                font_size: TILE_HEIGHT,
                ..default()
            },
            TextColor(color),
            BaseColor(color),
        )
    }

    pub(crate) fn get_urizen_sprite(&self, index: usize) -> Sprite {
        let mut sprite = Sprite::from_atlas_image(
            self.urizen.clone(),
            TextureAtlas {
                layout: self.urizen_layout.clone(),
                index,
            },
        );
        sprite.custom_size = Some(Vec2::new(TILE_WIDTH, TILE_HEIGHT));
        sprite
    }

    pub(crate) fn get_brainrot_sprite(&self) -> Sprite {
        use rand::seq::IndexedRandom;
        let mut rng = rand::rng();
        let index = self
            .valid_emote_indices
            .as_ref()
            .and_then(|indices| indices.choose(&mut rng).copied())
            .unwrap_or(0);

        let mut sprite = Sprite::from_atlas_image(
            self.ffz_tileset.clone(),
            TextureAtlas {
                layout: self.ffz_layout.clone(),
                index,
            },
        );
        sprite.custom_size = Some(Vec2::new(TILE_WIDTH, TILE_HEIGHT));
        sprite
    }

    pub(crate) fn get_urizen_egui_image(
        &'_ self,
        contexts: &mut EguiContexts,
        atlas_assets: &Assets<TextureAtlasLayout>,
        index: usize,
    ) -> egui::Image<'_> {
        get_egui_image_from_sprite(contexts, atlas_assets, &self.get_urizen_sprite(index))
    }

    pub(crate) fn get_solid_mask(&self) -> Handle<Image> {
        self.solid_mask.clone()
    }

    pub(crate) fn get_solid_hull(&self) -> Vec<Vec2> {
        let w = TILE_WIDTH / 2.0 * 0.4;
        let h = TILE_HEIGHT / 2.0 * 0.4;
        vec![
            Vec2::new(-w, -h),
            Vec2::new(w, -h),
            Vec2::new(w, h),
            Vec2::new(-w, h),
        ]
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

        let img_width = image.width();
        let img_height = image.height();

        let grid_layout = TextureAtlasLayout::from_grid(
            UVec2::splat(12),
            206,
            50,
            Some(UVec2::splat(1)),
            Some(UVec2::splat(1)),
        );
        let mut urizen_hulls = Vec::with_capacity(grid_layout.textures.len());

        for rect in &grid_layout.textures {
            let mut points = Vec::new();
            for y in rect.min.y..rect.max.y {
                for x in rect.min.x..rect.max.x {
                    let idx = (y as usize * img_width as usize + x as usize) * 4;
                    let alpha = data[idx + 3];
                    if alpha > 0 {
                        // Center the coordinates around the tile center
                        let local_x = x as f32 - rect.min.x as f32 - TILE_WIDTH / 2.0;
                        let local_y = y as f32 - rect.min.y as f32 - TILE_HEIGHT / 2.0;
                        points.push(Point::new(local_x as f64, local_y as f64));
                    }
                }
            }

            if points.is_empty() {
                urizen_hulls.push(Vec::new());
            } else {
                let multipoint = MultiPoint::from(points);
                let hull = multipoint.convex_hull();
                use geo::coords_iter::CoordsIter;
                let hull_vec2: Vec<Vec2> = hull
                    .exterior()
                    .coords_iter()
                    .map(|c| Vec2::new(c.x as f32, c.y as f32))
                    .collect();
                urizen_hulls.push(hull_vec2);
            }
        }

        let mask_image = Image::new(
            Extent3d {
                width: img_width,
                height: img_height,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            mask_data,
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::default(),
        );

        let mut tals = world.resource_mut::<Assets<TextureAtlasLayout>>();
        let urizen_layout = tals.add(grid_layout);
        let ffz_grid_layout = TextureAtlasLayout::from_grid(UVec2::splat(112), 32, 32, None, None);
        let ffz_layout = tals.add(ffz_grid_layout.clone());

        let mut images = world.resource_mut::<Assets<Image>>();
        let urizen = images.add(image);
        let urizen_mask = images.add(mask_image);

        let ffz_image = Image::from_buffer(
            include_bytes!("../../assets/emotes/ffz_tileset.png"),
            ImageType::Extension("png"),
            CompressedImageFormats::NONE,
            true,
            ImageSampler::Default,
            RenderAssetUsages::default() | RenderAssetUsages::MAIN_WORLD,
        )
        .expect("FFZ tileset should be valid");

        let mut valid_emote_indices = Vec::new();
        if let Some(ffz_data) = &ffz_image.data {
            let ffz_width = ffz_image.width() as usize;
            for (index, rect) in ffz_grid_layout.textures.iter().enumerate() {
                if FORBIDDEN_EMOTE_IDS.contains(&(index as u32)) {
                    continue;
                }

                let mut non_empty_pixels = 0;
                for y in rect.min.y..rect.max.y {
                    for x in rect.min.x..rect.max.x {
                        let pixel_idx = (y as usize * ffz_width + x as usize) * 4;
                        if ffz_data[pixel_idx + 3] > 0 {
                            non_empty_pixels += 1;
                        }
                    }
                }

                let total_pixels = (rect.max.x - rect.min.x) * (rect.max.y - rect.min.y);
                if non_empty_pixels * 2 >= total_pixels {
                    valid_emote_indices.push(index);
                }
            }
        }

        let ffz_tileset = images.add(ffz_image);
        let valid_emote_indices = Some(valid_emote_indices);

        let solid_mask_image = Image::new(
            Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            vec![255, 255, 255, 255],
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::default(),
        );
        let solid_mask = images.add(solid_mask_image);

        let font_bytes = include_bytes!("../../assets/PressStart2P/PressStart2P-Regular.ttf");
        let font_asset = Font::try_from_bytes(font_bytes.to_vec()).unwrap();

        // Calculate hulls for common ASCII characters
        let mut char_hulls = HashMap::new();
        use ab_glyph::{Font as _, FontArc, Glyph, PxScale, ScaleFont as _};
        let font_arc = FontArc::try_from_slice(font_bytes).unwrap();
        let scale = PxScale::from(TILE_HEIGHT);
        let scaled_font = font_arc.as_scaled(scale);

        for c in ' '..='~' {
            let glyph: Glyph = scaled_font
                .glyph_id(c)
                .with_scale_and_position(scale, ab_glyph::point(0.0, scaled_font.ascent()));
            if let Some(outline) = font_arc.outline_glyph(glyph) {
                let mut points = Vec::new();
                let bounds = outline.px_bounds();

                outline.draw(|x, y, v| {
                    if v > 0.5 {
                        // Center points around the middle of the character's bounding box
                        let px = x as f32 - bounds.width() / 2.0;
                        let py = y as f32 - bounds.height() / 2.0;
                        points.push(Point::new(px as f64, py as f64));
                    }
                });

                if !points.is_empty() {
                    let multipoint = MultiPoint::from(points);
                    let hull = multipoint.convex_hull();
                    use geo::coords_iter::CoordsIter;
                    let hull_vec2: Vec<Vec2> = hull
                        .exterior()
                        .coords_iter()
                        .map(|c| Vec2::new(c.x as f32, c.y as f32))
                        .collect();
                    char_hulls.insert(c, hull_vec2);
                }
            }
        }

        let font = world.resource_mut::<Assets<Font>>().add(font_asset);

        let comic_relief_bytes =
            include_bytes!("../../assets/Comic_Relief/ComicRelief-Regular.ttf");
        let comic_relief_asset = Font::try_from_bytes(comic_relief_bytes.to_vec()).unwrap();
        let comic_relief = world.resource_mut::<Assets<Font>>().add(comic_relief_asset);

        let comic_relief_bold_bytes =
            include_bytes!("../../assets/Comic_Relief/ComicRelief-Bold.ttf");
        let comic_relief_bold_asset =
            Font::try_from_bytes(comic_relief_bold_bytes.to_vec()).unwrap();
        let comic_relief_bold = world
            .resource_mut::<Assets<Font>>()
            .add(comic_relief_bold_asset);

        let phone_image = Image::from_buffer(
            include_bytes!("../../assets/mobile_app/phone.png"),
            ImageType::Extension("png"),
            CompressedImageFormats::NONE,
            true,
            ImageSampler::Default,
            RenderAssetUsages::default(),
        )
        .unwrap();
        let crawlr_image = Image::from_buffer(
            include_bytes!("../../assets/mobile_app/crawlr.png"),
            ImageType::Extension("png"),
            CompressedImageFormats::NONE,
            true,
            ImageSampler::Default,
            RenderAssetUsages::default(),
        )
        .unwrap();
        let dungeon_dash_image = Image::from_buffer(
            include_bytes!("../../assets/mobile_app/dungeon_dash.png"),
            ImageType::Extension("png"),
            CompressedImageFormats::NONE,
            true,
            ImageSampler::Default,
            RenderAssetUsages::default(),
        )
        .unwrap();
        let underground_tv_image = Image::from_buffer(
            include_bytes!("../../assets/mobile_app/underground_tv.png"),
            ImageType::Extension("png"),
            CompressedImageFormats::NONE,
            true,
            ImageSampler::Default,
            RenderAssetUsages::default(),
        )
        .unwrap();
        let cockatrice_image = Image::from_buffer(
            include_bytes!("../../assets/mobile_app/cockatrice.png"),
            ImageType::Extension("png"),
            CompressedImageFormats::NONE,
            true,
            ImageSampler::Default,
            RenderAssetUsages::default(),
        )
        .unwrap();

        let mut images = world.resource_mut::<Assets<Image>>();
        let phone = images.add(phone_image);
        let crawlr = images.add(crawlr_image);
        let dungeon_dash = images.add(dungeon_dash_image);
        let underground_tv = images.add(underground_tv_image);
        let cockatrice = images.add(cockatrice_image);

        let music = world
            .resource_mut::<Assets<AudioSource>>()
            .add(AudioSource {
                bytes: include_bytes!("../../assets/audio/music/brl_loop_v3.ogg")
                    .to_vec()
                    .into(),
            });

        let mut meme_sounds = Vec::new();
        let mut audio_assets = world.resource_mut::<Assets<AudioSource>>();
        for bytes in [
            include_bytes!("../../assets/audio/sound_effects/meme_sounds/cartoon run.ogg")
                .as_slice(),
            include_bytes!("../../assets/audio/sound_effects/meme_sounds/cartoon running.ogg")
                .as_slice(),
            include_bytes!("../../assets/audio/sound_effects/meme_sounds/he needs some milk.ogg")
                .as_slice(),
            include_bytes!(
                "../../assets/audio/sound_effects/meme_sounds/knuckles i don't know.ogg"
            )
            .as_slice(),
            include_bytes!("../../assets/audio/sound_effects/meme_sounds/wow!.ogg").as_slice(),
        ] {
            meme_sounds.push(audio_assets.add(AudioSource {
                bytes: bytes.to_vec().into(),
            }));
        }

        let oof = audio_assets.add(AudioSource {
            bytes: include_bytes!("../../assets/audio/sound_effects/meme_sounds/oof.ogg")
                .to_vec()
                .into(),
        });

        let button_click = audio_assets.add(AudioSource {
            bytes: include_bytes!("../../assets/audio/sound_effects/button_click.ogg")
                .to_vec()
                .into(),
        });

        let button_hover = audio_assets.add(AudioSource {
            bytes: include_bytes!("../../assets/audio/sound_effects/button_hover.ogg")
                .to_vec()
                .into(),
        });

        Self {
            font,
            comic_relief,
            comic_relief_bold,
            urizen,
            urizen_mask,
            urizen_layout,
            ffz_tileset,
            ffz_layout,
            valid_emote_indices,
            urizen_hulls,
            char_hulls,
            solid_mask,
            phone,
            phone_app_icons: PhoneAppIcons {
                crawlr,
                dungeon_dash,
                underground_tv,
                cockatrice,
            },
            music,
            meme_sounds,
            oof,
            button_click,
            button_hover,
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
    let [r, g, b, a] = sprite.color.to_srgba().to_u8_array();
    egui::Image::new(sized_texture)
        .uv(uv)
        .tint(Color32::from_rgba_unmultiplied(r, g, b, a))
}
