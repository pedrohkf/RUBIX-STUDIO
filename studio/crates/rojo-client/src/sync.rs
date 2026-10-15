//! Constrói o DataModel a partir da leitura inicial e aplica os patches do WebSocket.
//!
//! Regra de ouro: patch inconsistente vira `Error`, nunca pânico. Quem chama reage a erro
//! relendo a árvore inteira (`GET /api/read`), que é sempre seguro.

use std::collections::{HashMap, HashSet, VecDeque};

use dom::{InstanceBuilder, Ustr, WeakDom};
use rbx_types::{Ref, Variant};

use crate::error::{Error, Result};
use crate::types::{ReadResponse, SubscribeMessage, WireInstance};

fn builder_for(instance: &WireInstance) -> InstanceBuilder {
    InstanceBuilder::new(instance.class_name.as_str())
        .with_name(instance.name.as_str())
        .with_referent(instance.id)
        .with_properties(
            instance
                .properties
                .iter()
                .map(|(name, value)| (Ustr::from(name.as_str()), value.clone())),
        )
}

/// Monta a árvore completa a partir de `GET /api/read/{root}`.
///
/// Usa os mesmos `Ref` do servidor, o que torna a aplicação de patches direta.
pub fn build_dom(read: &ReadResponse, root: Ref) -> Result<WeakDom> {
    let root_instance = read.instances.get(&root).ok_or(Error::MissingRoot(root))?;
    let mut dom = WeakDom::new(builder_for(root_instance));

    // Largura primeiro: o pai sempre existe antes dos filhos.
    let mut queue: VecDeque<(Ref, Ref)> = VecDeque::new();
    for child in &root_instance.children {
        queue.push_back((root, *child));
    }
    let mut seen: HashSet<Ref> = HashSet::new();
    seen.insert(root);

    while let Some((parent, id)) = queue.pop_front() {
        if !seen.insert(id) {
            // Já inserida por outro caminho: o servidor mandou grafo, não árvore.
            return Err(Error::DuplicateInstance(id));
        }
        let instance = read.instances.get(&id).ok_or(Error::MissingInstance(id))?;
        dom.insert(parent, builder_for(instance));
        for child in &instance.children {
            queue.push_back((id, *child));
        }
    }
    Ok(dom)
}

/// Aplica um patch na ordem removidas, adicionadas, atualizadas.
///
/// (A ordem exata do plugin oficial ainda precisa ser confirmada; como os conjuntos de IDs
/// são disjuntos num mesmo patch, a ordem não muda o resultado.)
pub fn apply_message(dom: &mut WeakDom, message: &SubscribeMessage) -> Result<()> {
    let root = dom.root_ref();

    for id in &message.removed {
        if *id == root {
            return Err(Error::RemovedRoot);
        }
        if dom.get_by_ref(*id).is_some() {
            dom.destroy(*id);
        }
        // Já removida junto com um ancestral: não é erro.
    }

    add_instances(dom, &message.added)?;

    for update in &message.updated {
        let instance = dom
            .get_by_ref_mut(update.id)
            .ok_or(Error::UnknownInstance(update.id))?;
        if let Some(name) = &update.changed_name {
            instance.name = name.clone();
        }
        if let Some(class) = &update.changed_class_name {
            instance.class = Ustr::from(class.as_str());
        }
        for (property, value) in &update.changed_properties {
            let key = Ustr::from(property.as_str());
            match value {
                Some(value) => {
                    instance.properties.insert(key, value.clone());
                }
                None => {
                    instance.properties.remove(&key);
                }
            }
        }
    }
    Ok(())
}

fn add_instances(dom: &mut WeakDom, added: &HashMap<Ref, WireInstance>) -> Result<()> {
    if added.is_empty() {
        return Ok(());
    }
    for id in added.keys() {
        if dom.get_by_ref(*id).is_some() {
            return Err(Error::DuplicateInstance(*id));
        }
    }

    // Agrupa por pai: a ordem do mapa não é garantida.
    let mut by_parent: HashMap<Ref, Vec<Ref>> = HashMap::new();
    for instance in added.values() {
        by_parent
            .entry(instance.parent)
            .or_default()
            .push(instance.id);
    }

    // Raízes do patch = instâncias cujo pai já está na árvore (não em `added`).
    let mut queue: VecDeque<Ref> = VecDeque::new();
    for instance in added.values() {
        if !added.contains_key(&instance.parent) {
            if dom.get_by_ref(instance.parent).is_none() {
                continue; // contada como órfã abaixo
            }
            queue.push_back(instance.id);
        }
    }

    let mut inserted = 0usize;
    while let Some(id) = queue.pop_front() {
        let Some(instance) = added.get(&id) else {
            continue;
        };
        dom.insert(instance.parent, builder_for(instance));
        inserted += 1;
        if let Some(children) = by_parent.get(&id) {
            queue.extend(children.iter().copied());
        }
    }

    if inserted != added.len() {
        return Err(Error::OrphanInstances {
            count: added.len() - inserted,
        });
    }
    Ok(())
}

/// Valor de propriedade de uma Instance, se existir (atalho para testes e para o render).
pub fn property<'a>(dom: &'a WeakDom, id: Ref, name: &str) -> Option<&'a Variant> {
    dom.get_by_ref(id)?.properties.get(&Ustr::from(name))
}
