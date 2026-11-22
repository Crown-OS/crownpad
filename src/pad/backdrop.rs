use crownui::prelude::{Color, Material, MaterialBlur};

use crate::settings::Backdrop;

const LIGHT_TINT: Color = Color::new(0.96, 0.97, 0.98, 1.0);
const SOLID_ALPHA: f32 = 0.9;

pub fn preset(backdrop: Backdrop) -> Material {
    match backdrop {
        Backdrop::Thin => Material::thin(),
        Backdrop::Thick => Material::thick(),
    }
}

pub fn material(backdrop: Backdrop, reveal: f32, dark: bool) -> Material {
    let strength = reveal.clamp(0.0, 1.0).powi(2);
    let blur = preset(backdrop).blur.map(|blur| MaterialBlur {
        radius: blur.radius * strength,
        tint: Color {
            a: blur.tint.a * strength,
            ..tint(blur.tint, dark)
        },
        saturation: 1.0 + (blur.saturation - 1.0) * strength,
    });
    Material {
        corner_radius: 0.0,
        border_width: 0.0,
        blur,
        shadow: None,
    }
}

pub fn solid(backdrop: Backdrop, dark: bool) -> Color {
    let base = preset(backdrop)
        .blur
        .map_or(Color::new(0.12, 0.13, 0.17, 1.0), |blur| blur.tint);
    Color {
        a: SOLID_ALPHA,
        ..tint(base, dark)
    }
}

pub fn tray_tint(dark: bool) -> (f32, Color) {
    let thin = Material::thin();
    let blur = thin
        .blur
        .map_or((0.0, Color::new(0.12, 0.13, 0.17, 0.25)), |blur| {
            (blur.radius, blur.tint)
        });
    (blur.0, tint(blur.1, dark))
}

fn tint(color: Color, dark: bool) -> Color {
    if dark {
        color
    } else {
        Color {
            a: color.a,
            ..LIGHT_TINT
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hidden_pad_requests_no_blur() {
        let material = material(Backdrop::Thick, 0.0, true);
        assert_eq!(material.blur.map(|blur| blur.radius), Some(0.0));
        assert_eq!(material.blur.map(|blur| blur.tint.a), Some(0.0));
    }

    #[test]
    fn revealed_pad_uses_the_crownui_preset() {
        let material = material(Backdrop::Thick, 1.0, true);
        assert_eq!(material.blur, Material::thick().blur);
    }
}
