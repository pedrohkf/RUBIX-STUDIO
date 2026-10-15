//! Câmera isométrica fixa do jogo-alvo. Matemática pura, testável sem janela.

use bevy::prelude::*;

pub const PITCH_DEG: f32 = 55.0;
pub const YAW_DEG: f32 = 45.0;
pub const FOV_DEG: f32 = 30.0;
pub const FOCUS_HEIGHT: f32 = 2.0;
pub const DEFAULT_DISTANCE: f32 = 70.0;
pub const MIN_DISTANCE: f32 = 40.0;
pub const MAX_DISTANCE: f32 = 110.0;
pub const ZOOM_STEP: f32 = 5.0;
pub const FOLLOW_SHARPNESS: f32 = 12.0;

/// Posição da câmera para um foco e distância dados.
pub fn camera_position(focus: Vec3, distance: f32) -> Vec3 {
    let pitch = PITCH_DEG.to_radians();
    let yaw = YAW_DEG.to_radians();
    let horizontal = distance * pitch.cos();
    focus
        + Vec3::new(
            horizontal * yaw.sin(),
            distance * pitch.sin(),
            horizontal * yaw.cos(),
        )
}

/// Zoom em passos de 5 studs, preso entre 40 e 110. `scroll > 0` aproxima.
pub fn zoom(distance: f32, scroll: f32) -> f32 {
    let steps = scroll.round();
    (distance - steps * ZOOM_STEP).clamp(MIN_DISTANCE, MAX_DISTANCE)
}

/// Suavização exponencial independente de FPS.
pub fn follow(current: Vec3, target: Vec3, dt: f32) -> Vec3 {
    current.lerp(target, 1.0 - (-FOLLOW_SHARPNESS * dt).exp())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn a3_camera_geometry() {
        let focus = Vec3::new(0.0, FOCUS_HEIGHT, 0.0);
        let pos = camera_position(focus, DEFAULT_DISTANCE);
        assert!((pos.distance(focus) - 70.0).abs() < 1e-3);
        let offset = pos - focus;
        let pitch = (offset.y / offset.length()).asin().to_degrees();
        assert!((pitch - 55.0).abs() < 1e-3);
        let yaw = offset.x.atan2(offset.z).to_degrees();
        assert!((yaw - 45.0).abs() < 1e-3);
    }

    #[test]
    fn a4_zoom_steps_and_limits() {
        assert_eq!(zoom(70.0, 1.0), 65.0);
        assert_eq!(zoom(70.0, -2.0), 80.0);
        assert_eq!(zoom(40.0, 3.0), 40.0);
        assert_eq!(zoom(110.0, -3.0), 110.0);
    }

    #[test]
    fn a5_follow_converges_without_overshoot() {
        let mut p = Vec3::ZERO;
        let target = Vec3::new(10.0, 0.0, 0.0);
        for _ in 0..60 {
            p = follow(p, target, 1.0 / 60.0);
            assert!(p.x <= target.x + 1e-4);
        }
        assert!((p.x - 10.0).abs() < 0.01);
    }
}
