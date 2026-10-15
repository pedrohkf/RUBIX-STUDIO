//! Testes contra respostas REAIS do `rojo serve` 7.7.1 (protocolo 5), capturadas com
//! `harness/capture_rojo_fixtures.py` a partir de uma cópia de harness/sample-game.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::path::PathBuf;

use dom::{WeakDom, dump};
use rojo_client::sync::{apply_message, build_dom, property};
use rojo_client::types::{ReadResponse, ServerInfo, SocketPacket, decode};
use rojo_client::{Error, SUPPORTED_PROTOCOL_VERSION};

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../harness/fixtures/rojo")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|err| panic!("fixture {}: {err}", path.display()))
}

fn initial() -> (ServerInfo, WeakDom) {
    let info: ServerInfo = decode(&fixture("info.msgpack")).unwrap();
    let read: ReadResponse = decode(&fixture("read_before.msgpack")).unwrap();
    assert_eq!(read.session_id, info.session_id);
    let dom = build_dom(&read, info.root_instance_id).unwrap();
    (info, dom)
}

fn count(dom: &WeakDom) -> usize {
    dom.descendants().count()
}

fn find<'a>(dom: &'a WeakDom, name: &str) -> Option<&'a dom::rbx_dom_weak::Instance> {
    dom.descendants().find(|instance| instance.name == name)
}

#[test]
fn handshake_real_server_info() {
    let info: ServerInfo = decode(&fixture("info.msgpack")).unwrap();
    assert_eq!(info.protocol_version, SUPPORTED_PROTOCOL_VERSION);
    assert_eq!(info.server_version, "7.7.1");
    assert_eq!(info.project_name, "SampleGame");
    assert_eq!(info.root_instance_id.to_string().len(), 32);
    info.check_protocol().unwrap();
}

#[test]
fn wrong_protocol_version_fails_with_clear_error() {
    let mut info: ServerInfo = decode(&fixture("info.msgpack")).unwrap();
    info.protocol_version = 6;
    info.server_version = "8.0.0".into();
    let err = info.check_protocol().unwrap_err();
    assert!(matches!(
        err,
        Error::ProtocolMismatch {
            server_protocol: 6,
            ..
        }
    ));
    let text = err.to_string();
    assert!(
        text.contains("8.0.0") && text.contains("protocolo 6"),
        "{text}"
    );
}

#[test]
fn builds_the_whole_sample_tree() {
    let (info, dom) = initial();
    assert_eq!(dom.root().class.as_str(), "DataModel");
    assert_eq!(dom.root_ref(), info.root_instance_id);
    // A leitura real trouxe 12 instâncias; `descendants()` inclui a raiz.
    assert_eq!(count(&dom), 12);
    let main = find(&dom, "Main").expect("Main.server ou Main.client");
    assert!(main.class.as_str().ends_with("Script"));
    let source = main.properties.get(&"Source".into()).expect("Source");
    assert!(format!("{source:?}").starts_with("String("));
}

#[test]
fn real_patches_update_add_and_remove() {
    let (info, mut dom) = initial();
    let before = dump(&dom, true);
    let before_count = count(&dom);

    let apply = |dom: &mut WeakDom, name: &str| {
        let packet: SocketPacket = decode(&fixture(name)).unwrap();
        assert_eq!(packet.session_id, info.session_id);
        assert_eq!(packet.packet_type, "messages");
        for message in &packet.body.messages {
            apply_message(dom, message).unwrap();
        }
        packet.body.message_cursor
    };

    // 1) atualização de Source de HealthColor
    let c1 = apply(&mut dom, "patch_1_update.msgpack");
    let health = find(&dom, "HealthColor").expect("HealthColor");
    let health_id = health.referent();
    let source = format!("{:?}", property(&dom, health_id, "Source").unwrap());
    assert!(source.contains("-- editado pelo fixture"), "{source}");
    assert_eq!(count(&dom), before_count);
    let changed_lines = dump(&dom, true)
        .lines()
        .zip(before.lines())
        .filter(|(a, b)| a != b)
        .count();
    assert_eq!(changed_lines, 1, "só o Source de HealthColor deveria mudar");

    // 2) arquivo novo vira uma instância
    let c2 = apply(&mut dom, "patch_2_add.msgpack");
    assert_eq!(count(&dom), before_count + 1);
    let added = find(&dom, "FixtureNew").expect("FixtureNew");
    assert_eq!(added.class.as_str(), "ModuleScript");
    let parent = dom.get_by_ref(added.parent()).unwrap();
    assert_eq!(parent.name, "Shared");
    let added_source = format!("{:?}", property(&dom, added.referent(), "Source").unwrap());
    assert!(added_source.contains("fixture = true"), "{added_source}");

    // 3) arquivo apagado some da árvore
    let c3 = apply(&mut dom, "patch_3_remove.msgpack");
    assert_eq!(count(&dom), before_count);
    assert!(find(&dom, "FixtureNew").is_none());

    assert!(c1 < c2 && c2 < c3, "o cursor deve avançar: {c1} {c2} {c3}");
}

#[test]
fn full_read_after_restore_matches_the_initial_tree() {
    let (_, dom) = initial();
    let info: ServerInfo = decode(&fixture("info.msgpack")).unwrap();
    let read: ReadResponse = decode(&fixture("read_after.msgpack")).unwrap();
    let after = build_dom(&read, info.root_instance_id).unwrap();
    assert_eq!(dump(&dom, true), dump(&after, true));
}

#[test]
fn inconsistent_patches_are_errors_not_panics() {
    use rojo_client::types::{InstanceUpdate, SubscribeMessage};
    let (info, mut dom) = initial();

    let removed_root = SubscribeMessage {
        removed: vec![info.root_instance_id],
        ..Default::default()
    };
    assert!(matches!(
        apply_message(&mut dom, &removed_root),
        Err(Error::RemovedRoot)
    ));

    let unknown = SubscribeMessage {
        updated: vec![InstanceUpdate {
            id: rbx_types::Ref::new(),
            changed_name: Some("x".into()),
            changed_class_name: None,
            changed_properties: Default::default(),
            changed_metadata: None,
        }],
        ..Default::default()
    };
    assert!(matches!(
        apply_message(&mut dom, &unknown),
        Err(Error::UnknownInstance(_))
    ));
}
