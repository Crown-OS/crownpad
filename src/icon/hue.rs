const MIN_ALPHA: u8 = 200;
const MIN_SATURATION: f32 = 0.25;
const MIN_VALUE: f32 = 0.2;
const SAMPLE_STRIDE: usize = 4;

pub fn dominant_hue(rgba: &[u8]) -> Option<f32> {
    let (sum_x, sum_y, count) = rgba
        .chunks_exact(4)
        .step_by(SAMPLE_STRIDE)
        .filter(|pixel| pixel[3] >= MIN_ALPHA)
        .filter_map(|pixel| hue_of(pixel[0], pixel[1], pixel[2]))
        .fold((0.0f32, 0.0f32, 0u32), |(x, y, count), hue| {
            let radians = hue.to_radians();
            (x + radians.cos(), y + radians.sin(), count + 1)
        });
    (count > 0).then(|| sum_y.atan2(sum_x).to_degrees().rem_euclid(360.0))
}

fn hue_of(red: u8, green: u8, blue: u8) -> Option<f32> {
    let [red, green, blue] = [red, green, blue].map(|channel| f32::from(channel) / 255.0);
    let max = red.max(green).max(blue);
    let delta = max - red.min(green).min(blue);
    if max < MIN_VALUE || delta / max < MIN_SATURATION {
        return None;
    }
    let sector = if max == red {
        (green - blue) / delta
    } else if max == green {
        (blue - red) / delta + 2.0
    } else {
        (red - green) / delta + 4.0
    };
    Some((sector * 60.0).rem_euclid(360.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(red: u8, green: u8, blue: u8) -> Vec<u8> {
        [red, green, blue, 255].repeat(64)
    }

    #[test]
    fn primary_colours_have_their_hue() {
        let hue = |rgba: Vec<u8>| dominant_hue(&rgba).map(f32::round);
        assert_eq!(hue(solid(255, 0, 0)), Some(0.0));
        assert_eq!(hue(solid(0, 255, 0)), Some(120.0));
        assert_eq!(hue(solid(0, 0, 255)), Some(240.0));
    }

    #[test]
    fn grey_and_transparent_icons_have_no_hue() {
        assert_eq!(dominant_hue(&solid(128, 128, 128)), None);
        assert_eq!(dominant_hue(&[255, 0, 0, 0].repeat(64)), None);
    }
}
