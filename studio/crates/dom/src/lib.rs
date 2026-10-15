//! DataModel da engine: fonte de verdade da árvore de Instances.
//!
//! Usa `rbx_dom_weak` (MIT). Esta camada fica fina de propósito: o resto da engine
//! (render, física, Luau) lê e escreve a árvore por aqui.

use std::fmt::Write as _;

pub use rbx_dom_weak;
pub use rbx_dom_weak::{
    InstanceBuilder, Ustr, UstrMap, WeakDom,
    types::{Ref, Variant},
};

/// Texto canônico da árvore, para comparar com o oráculo (Lune) e ler em testes.
///
/// Uma linha por Instance: `<indentação><Classe> "<Nome>"`. Com `with_properties`,
/// as propriedades vêm logo abaixo, ordenadas por nome. A ordem dos filhos é a da árvore
/// (importa no Roblox).
pub fn dump(dom: &WeakDom, with_properties: bool) -> String {
    let mut out = String::new();
    dump_instance(dom, dom.root_ref(), 0, with_properties, &mut out);
    out
}

fn dump_instance(dom: &WeakDom, id: Ref, depth: usize, with_properties: bool, out: &mut String) {
    let Some(instance) = dom.get_by_ref(id) else {
        return;
    };
    let indent = "  ".repeat(depth);
    let _ = writeln!(out, "{indent}{} {:?}", instance.class, instance.name);
    if with_properties {
        let mut names: Vec<_> = instance.properties.keys().collect();
        names.sort();
        for name in names {
            if let Some(value) = instance.properties.get(name) {
                let _ = writeln!(out, "{indent}  .{name} = {value:?}");
            }
        }
    }
    for child in instance.children() {
        dump_instance(dom, *child, depth + 1, with_properties, out);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn dump_keeps_child_order_and_sorts_properties() {
        let dom = WeakDom::new(
            InstanceBuilder::new("DataModel")
                .with_name("Game")
                .with_child(
                    InstanceBuilder::new("Workspace")
                        .with_name("Workspace")
                        .with_child(
                            InstanceBuilder::new("Part")
                                .with_name("B")
                                .with_property("Anchored", true)
                                .with_property("Transparency", 0.5f32),
                        )
                        .with_child(InstanceBuilder::new("Part").with_name("A")),
                ),
        );
        let text = dump(&dom, true);
        assert_eq!(
            text,
            "DataModel \"Game\"\n  Workspace \"Workspace\"\n    Part \"B\"\n      .Anchored = Bool(true)\n      .Transparency = Float32(0.5)\n    Part \"A\"\n"
        );
    }
}
