use crate::game::{Player, map::MapPos};
use bevy::{platform::collections::HashMap, prelude::*};
use noisy_bevy::simplex_noise_2d_seeded;

#[derive(Component, Debug, Clone)]
pub struct SignalMap {
    pub bars: HashMap<IVec2, i32>,
}

pub fn generate_signal_map(
    rect: rogue_algebra::Rect,
    seed: u32,
    strength: f32,
    frequency: f32,
) -> SignalMap {
    let mut bars = HashMap::default();

    let max_min_dist = (rect.height() as f32 - 1.0) / 2.0;

    for x in rect.x1..=rect.x2 {
        for y in rect.y1..=rect.y2 {
            let n = simplex_noise_2d_seeded(
                Vec2::new(x as f32 * frequency, y as f32 * frequency),
                seed as f32,
            );

            let dist_to_edge_x = (x - rect.x1).min(rect.x2 - x) as f32;
            let dist_to_edge_y = (y - rect.y1).min(rect.y2 - y) as f32;
            let min_dist_to_edge = dist_to_edge_x.min(dist_to_edge_y);

            // e = 0.0 at center, 1.0 at closest edge
            let e = (1.0 - (min_dist_to_edge / max_min_dist)).clamp(0.0, 1.0);

            // 0.5 at center (2.5 bars average), 0.9 at edges (4.5 bars average)
            let base = e * 0.5 + 0.5;
            // n * 0.5 adds +/- 2.5 bars of oscillation
            let s = ((base + n * 0.5).clamp(0.0, 1.0) * strength).clamp(0.0, 1.0);

            let signal_bars = (s * 5.0).round() as i32;
            bars.insert(IVec2::new(x, y), signal_bars);
        }
    }

    SignalMap { bars }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rogue_algebra::Rect;

    #[test]
    fn test_signal_map_variation() {
        let rect = Rect::new(0, 20, 0, 20);
        let seed = 12345;
        let signal_map = generate_signal_map(rect, seed, 1.0, 0.1);

        let mut counts = [0; 6];
        let mut center_sum = 0;
        let mut edge_sum = 0;
        let mut center_count = 0;
        let mut edge_count = 0;

        let center = IVec2::new(10, 10);

        for (pos, &bars) in signal_map.bars.iter() {
            if (bars as usize) < counts.len() {
                counts[bars as usize] += 1;
            }

            let dist_to_center = (pos.x - center.x).abs().max((pos.y - center.y).abs());
            if dist_to_center <= 3 {
                center_sum += bars;
                center_count += 1;
            } else if dist_to_center >= 8 {
                edge_sum += bars;
                edge_count += 1;
            }
        }

        // Ensure there is variation
        let unique_values = counts.iter().filter(|&&c| c > 0).count();
        assert!(
            unique_values > 1,
            "Signal map should have more than one unique value"
        );

        let center_avg = center_sum as f32 / center_count as f32;
        let edge_avg = edge_sum as f32 / edge_count as f32;

        // Edges should generally have higher signal than the center
        assert!(
            edge_avg > center_avg,
            "Edges should have higher average signal than center"
        );
    }
}

pub fn update_player_signal(
    player: Single<(&mut Player, &MapPos)>,
    signal_maps: Query<&SignalMap>,
) {
    let (mut player, pos) = player.into_inner();

    if player.has_subscription(crate::game::Subscription::FiveGLTE) {
        player.signal = 5;
        return;
    }

    for signal_map in signal_maps.iter() {
        if let Some(&bars) = signal_map.bars.get(&pos.0) {
            player.signal = bars;
            return;
        }
    }
}
