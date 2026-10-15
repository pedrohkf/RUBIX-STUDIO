//! Leitura das Parts do Workspace para o renderer (o app não conhece o Lune).
//!
//! Limitação conhecida (docs/decisions.md): no `lune-roblox`, `Position` e `CFrame` são propriedades
//! independentes. Aqui, `Position`, se existir, vale sobre a posição do `CFrame`.

use lune_roblox::instance::Instance;
use rbx_types::{Ref, Variant};

use crate::Runtime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartShape {
    Ball,
    Block,
    Cylinder,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PartView {
    pub id: Ref,
    pub name: String,
    pub position: [f32; 3],
    /// Matriz de rotação por colunas (eixos X, Y e Z da Part no mundo).
    pub rotation: [[f32; 3]; 3],
    pub size: [f32; 3],
    pub color: [f32; 3],
    pub transparency: f32,
    pub shape: PartShape,
}

// Padrões de uma Part nova no Roblox (a confirmar contra rbx_reflection_database).
const DEFAULT_SIZE: [f32; 3] = [4.0, 1.0, 2.0];
const DEFAULT_COLOR: [f32; 3] = [0.639, 0.635, 0.647];

fn part_view(part: &Instance) -> PartView {
    let mut position = [0.0; 3];
    let mut rotation = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    if let Some(Variant::CFrame(cf)) = part.get_property("CFrame") {
        position = [cf.position.x, cf.position.y, cf.position.z];
        // rbx_types guarda as LINHAS da matriz; o renderer quer as colunas (eixos da Part).
        let (x, y, z) = (cf.orientation.x, cf.orientation.y, cf.orientation.z);
        rotation = [[x.x, y.x, z.x], [x.y, y.y, z.y], [x.z, y.z, z.z]];
    }
    if let Some(Variant::Vector3(p)) = part.get_property("Position") {
        position = [p.x, p.y, p.z];
    }
    let size = match part.get_property("Size") {
        Some(Variant::Vector3(s)) => [s.x, s.y, s.z],
        _ => DEFAULT_SIZE,
    };
    let color = match part.get_property("Color") {
        Some(Variant::Color3(c)) => [c.r, c.g, c.b],
        _ => DEFAULT_COLOR,
    };
    let transparency = match part.get_property("Transparency") {
        Some(Variant::Float32(t)) => t,
        _ => 0.0,
    };
    let shape = match part.get_property("Shape") {
        Some(Variant::Enum(e)) => match e.to_u32() {
            0 => PartShape::Ball,
            2 => PartShape::Cylinder,
            _ => PartShape::Block,
        },
        _ => PartShape::Block,
    };
    PartView {
        id: part.dom_ref,
        name: part.get_name(),
        position,
        rotation,
        size,
        color,
        transparency,
        shape,
    }
}

impl Runtime {
    /// Todas as Parts (BasePart) dentro do Workspace, na ordem da árvore.
    pub fn workspace_parts(&self) -> Vec<PartView> {
        let Some(workspace) = self.game().find_child(|c| c.class == "Workspace") else {
            return Vec::new();
        };
        workspace
            .get_descendants_preorder()
            .iter()
            .filter(|i| i.is_a("BasePart"))
            .map(part_view)
            .collect()
    }
}
