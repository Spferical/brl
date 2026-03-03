use crate::game::{Player, map::MapPos};
use bevy::{platform::collections::HashMap, prelude::*};
use noise::{NoiseFn, Perlin};

#[derive(Component, Debug, Clone)]
pub struct SignalMap {
    pub bars: HashMap<IVec2, i32>,
}

pub fn generate_signal_map(width: i32, height: i32, seed: u32, offset: IVec2) -> SignalMap {
    let perlin = Perlin::new(seed);
    let mut bars = HashMap::default();

    let max_min_dist = (height as f32 - 1.0) / 2.0;

    for x in 0..width {
        for y in 0..height {
            let nx = x as f64 * 1.5;
            let ny = y as f64 * 1.5;

            let n = perlin.get([nx, ny, seed as f64]);

            let dist_to_edge_x = x.min(width - 1 - x) as f32;
            let dist_to_edge_y = y.min(height - 1 - y) as f32;
            let min_dist_to_edge = dist_to_edge_x.min(dist_to_edge_y);

            // e = 0.0 at center, 1.0 at closest edge
            let e = (1.0 - (min_dist_to_edge / max_min_dist)).clamp(0.0, 1.0);

            // 0.4 at center (2 bars average), 0.9 at edges (4.5 bars average)
            let base = e * 0.5 + 0.4;
            // n * 0.25 adds +/- 1.25 bars of oscillation
            let s = (base + n as f32 * 0.25).clamp(0.0, 1.0);

            let signal_bars = (s * 5.0).round() as i32;
            bars.insert(offset + IVec2::new(x, y), signal_bars);
        }
    }

    SignalMap { bars }
}

pub fn update_player_signal(
    player: Single<(&mut Player, &MapPos)>,
    signal_maps: Query<&SignalMap>,
) {
    let (mut player, pos) = player.into_inner();

    for signal_map in signal_maps.iter() {
        if let Some(&bars) = signal_map.bars.get(&pos.0) {
            player.signal = bars;
            return;
        }
    }
}
