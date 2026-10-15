//! Teste ao vivo: sobe `rojo serve` numa cópia do harness/sample-game, edita um arquivo e confere o patch (M3).
//! Pula (com aviso) se o `rojo` não estiver no PATH nem em ~/.cargo/bin.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use rojo_client::{Poll, RojoClient, Session};

struct Server(Child);
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn rojo_bin() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from)?;
    let candidates = [home.join(".cargo/bin/rojo"), PathBuf::from("rojo")];
    candidates
        .into_iter()
        .find(|path| Command::new(path).arg("--version").output().is_ok())
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

#[test]
fn live_edit_reaches_dom_under_one_second() {
    let Some(rojo) = rojo_bin() else {
        eprintln!("PULADO: rojo não encontrado");
        return;
    };
    let game = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../harness/sample-game");
    let work = std::env::temp_dir().join(format!("studio-live-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&work);
    copy_dir(&game, &work);

    let port = 34999;
    let child = Command::new(rojo)
        .args(["serve", "--port", &port.to_string()])
        .arg(work.join("default.project.json"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let _server = Server(child);

    let client = RojoClient::new(format!("http://127.0.0.1:{port}"));
    let start = Instant::now();
    let mut session = loop {
        match Session::connect(&client) {
            Ok(session) => break session,
            Err(err) if start.elapsed() < Duration::from_secs(20) => {
                let _ = err;
                std::thread::sleep(Duration::from_millis(200));
            }
            Err(err) => panic!("não conectou ao rojo serve: {err}"),
        }
    };
    let before = session.dom.descendants().count();
    assert!(before >= 12, "árvore pequena demais: {before}");

    std::fs::write(work.join("src/shared/ZzLiveProbe.luau"), "return 'live'\n").unwrap();
    let edited = Instant::now();
    let result = session.wait_patch(Duration::from_secs(5)).unwrap();
    let latency = edited.elapsed();
    assert!(matches!(result, Poll::Patched { .. }), "sem patch");
    assert!(
        session
            .dom
            .descendants()
            .any(|instance| instance.name == "ZzLiveProbe"),
        "ZzLiveProbe não apareceu na árvore"
    );
    eprintln!("latência arquivo→DataModel: {latency:?}");
    assert!(latency < Duration::from_secs(1), "lento: {latency:?}");
    let _ = std::fs::remove_dir_all(&work);
}
