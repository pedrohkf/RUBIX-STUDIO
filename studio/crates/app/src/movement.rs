//! Clique para andar do jogo-alvo (clique direito move o personagem). Lógica pura, sem Bevy.
//! Números do Config do jogo-alvo: Movement e PointClick.

use bevy::math::Vec3;

pub const WALK_SPEED: f32 = 16.0;
pub const SPRINT_MULTIPLIER: f32 = 1.5;
pub const MAX_CLICK_DISTANCE: f32 = 300.0;
pub const DOUBLE_CLICK_WINDOW: f64 = 0.35;
pub const DASH_DURATION: f64 = 0.4;
/// Distância em que o Humanoid considera o destino alcançado (provisório: a confirmar no Roblox).
pub const ARRIVE_EPSILON: f32 = 0.1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Click {
    /// Longe demais do jogador (> 300 studs): o servidor ignora.
    Ignored,
    Walk,
    Dash,
}

#[derive(Debug, Default)]
pub struct Mover {
    target: Option<Vec3>,
    last_click: Option<f64>,
    dash_until: f64,
}

impl Mover {
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn target(&self) -> Option<Vec3> {
        self.target
    }

    pub fn speed(&self, now: f64) -> f32 {
        if now < self.dash_until {
            WALK_SPEED * SPRINT_MULTIPLIER
        } else {
            WALK_SPEED
        }
    }

    /// Clique direito em `point` (no chão) com o jogador em `player`.
    pub fn click(&mut self, now: f64, player: Vec3, point: Vec3) -> Click {
        // O cliente decide o clique duplo antes de saber se o servidor aceita o ponto.
        let double = self
            .last_click
            .is_some_and(|last| now - last < DOUBLE_CLICK_WINDOW);
        self.last_click = Some(now);
        if !point.is_finite() || player.distance(point) > MAX_CLICK_DISTANCE {
            return Click::Ignored;
        }
        self.target = Some(Vec3::new(point.x, player.y, point.z));
        if double {
            self.dash_until = now + DASH_DURATION;
            Click::Dash
        } else {
            Click::Walk
        }
    }

    /// Avança `dt` segundos. Movimento só no plano horizontal; devolve a nova posição.
    pub fn step(&mut self, now: f64, position: Vec3, dt: f32) -> Vec3 {
        let Some(target) = self.target else {
            return position;
        };
        let flat_target = Vec3::new(target.x, position.y, target.z);
        let to_target = flat_target - position;
        let distance = to_target.length();
        let travel = self.speed(now) * dt;
        if distance <= travel.max(ARRIVE_EPSILON) {
            self.target = None;
            return flat_target;
        }
        position + to_target / distance * travel
    }
}

/// Interseção de um raio com o plano y = `height` (chão). `None` se o raio não desce.
pub fn ray_ground(origin: Vec3, direction: Vec3, height: f32) -> Option<Vec3> {
    if direction.y >= -1e-6 {
        return None;
    }
    let t = (height - origin.y) / direction.y;
    (t >= 0.0).then(|| origin + direction * t)
}

/// Bolinha do clique: tamanho e transparência em `t` segundos (Tween Quad Out de 0,5 s).
pub mod marker {
    pub const SIZE: f32 = 2.4;
    pub const THICKNESS: f32 = 0.2;
    pub const LIFETIME: f32 = 0.5;
    pub const START_TRANSPARENCY: f32 = 0.3;

    fn quad_out(x: f32) -> f32 {
        let x = x.clamp(0.0, 1.0);
        1.0 - (1.0 - x) * (1.0 - x)
    }

    /// (diâmetro, transparência) em `t`; `None` depois do tempo de vida (Destroy).
    pub fn at(t: f32) -> Option<(f32, f32)> {
        if t >= LIFETIME {
            return None;
        }
        let k = quad_out(t / LIFETIME);
        Some((
            SIZE + SIZE * k,
            START_TRANSPARENCY + (1.0 - START_TRANSPARENCY) * k,
        ))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;

    /// Simula até parar e devolve (posição final, tempo gasto).
    fn run(mover: &mut Mover, mut now: f64, mut pos: Vec3) -> (Vec3, f64) {
        let start = now;
        for _ in 0..100_000 {
            if mover.target().is_none() {
                break;
            }
            pos = mover.step(now, pos, DT);
            now += f64::from(DT);
        }
        (pos, now - start)
    }

    #[test]
    fn a6_a7_walks_to_click_at_16_studs_per_second() {
        let mut mover = Mover::default();
        let start = Vec3::new(0.0, 3.0, 0.0);
        assert_eq!(
            mover.click(0.0, start, Vec3::new(48.0, 0.0, 0.0)),
            Click::Walk
        );
        let (pos, time) = run(&mut mover, 0.0, start);
        assert!(pos.distance(Vec3::new(48.0, 3.0, 0.0)) < 1.0);
        // 48 studs ÷ 16 = 3 s (margem de 1 quadro).
        assert!((time - 3.0).abs() <= f64::from(DT) * 1.5, "tempo {time}");
    }

    #[test]
    fn a8_double_click_dashes_at_24_for_0_4_s() {
        let mut mover = Mover::default();
        let p = Vec3::ZERO;
        let far = Vec3::new(100.0, 0.0, 0.0);
        assert_eq!(mover.click(0.0, p, far), Click::Walk);
        assert_eq!(mover.click(0.30, p, far), Click::Dash);
        assert_eq!(mover.speed(0.31), 24.0);
        assert_eq!(mover.speed(0.30 + DASH_DURATION + 0.01), 16.0);
        // Fora da janela de 0,35 s não é duplo.
        assert_eq!(mover.click(1.0, p, far), Click::Walk);
    }

    #[test]
    fn a10_click_beyond_300_is_ignored() {
        let mut mover = Mover::default();
        assert_eq!(
            mover.click(0.0, Vec3::ZERO, Vec3::new(301.0, 0.0, 0.0)),
            Click::Ignored
        );
        assert!(mover.target().is_none());
        assert_eq!(
            mover.click(1.0, Vec3::ZERO, Vec3::new(f32::NAN, 0.0, 0.0)),
            Click::Ignored
        );
    }

    #[test]
    fn a9_without_click_nothing_moves() {
        let mut mover = Mover::default();
        let p = Vec3::new(5.0, 3.0, 5.0);
        assert_eq!(mover.step(0.0, p, DT), p);
    }

    #[test]
    fn ray_hits_ground_plane() {
        let hit = ray_ground(Vec3::new(0.0, 10.0, 0.0), Vec3::new(1.0, -1.0, 0.0), 0.0).unwrap();
        assert!(hit.distance(Vec3::new(10.0, 0.0, 0.0)) < 1e-4);
        assert!(ray_ground(Vec3::Y, Vec3::Y, 0.0).is_none());
    }

    #[test]
    fn a6_marker_grows_to_double_and_fades() {
        let (size, transparency) = marker::at(0.0).unwrap();
        assert_eq!((size, transparency), (2.4, 0.3));
        let (size, transparency) = marker::at(0.4999).unwrap();
        assert!((size - 4.8).abs() < 0.01 && (transparency - 1.0).abs() < 0.01);
        assert!(marker::at(0.5).is_none());
    }
}
