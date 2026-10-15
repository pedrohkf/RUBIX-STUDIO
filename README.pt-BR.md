# RUBIX Studio

[English](README.md)

Um **Studio leve para desenvolver e rodar jogos**, escrito em Rust, para Linux.

A ideia é simples: o fluxo de trabalho do Roblox Studio (scripts em Luau, árvore de Instances, sincronização com [Rojo](https://github.com/rojo-rbx/rojo) e editor de código à escolha) funcionando em Linux, sem depender do Roblox para jogar e testar. Você escreve o jogo como código, a engine abre uma janela, roda os scripts e mostra o resultado em 3D.

## Por que existe

- O Roblox Studio não roda no Linux. Quem desenvolve nele precisa de Windows, macOS ou gambiarras.
- Para um jogo feito só de código (como o RUBIX, jogo-alvo deste projeto), carregar um editor completo é pesado. O que se quer é um ciclo curto: salvar o arquivo, ver o resultado em menos de um segundo.
- Um runtime pequeno, aberto e de código legível serve também para testar a lógica do jogo com testes automáticos, sem abrir janela.

Não é um clone do Roblox e **não promete paridade**. É um runtime compatível com o subconjunto da API que os jogos-alvo realmente usam, e esse subconjunto cresce conforme novos jogos são testados. As diferenças conhecidas ficam registradas em `studio/docs/`.

## Como funciona

```
 arquivos do jogo ──► rojo serve ──► cliente Rojo (Rust) ──► DataModel ──► Luau (mlua) ──► render (Bevy/Vulkan)
```

| Crate | O que faz |
|---|---|
| `studio/crates/dom` | DataModel: árvore de Instances (sobre `rbx_dom_weak`). |
| `studio/crates/rojo-client` | Cliente do protocolo do Rojo (v5): handshake, leitura da árvore, WebSocket e patches ao vivo. |
| `studio/crates/lua` | Ponte Luau: `Instance`, tipos de dados, `require`, `task.*` com relógio virtual para testes determinísticos. |
| `studio/crates/app` | Janela Bevy: chão, câmera isométrica, clique para andar. |

Stack: Rust, [Bevy](https://bevyengine.org) (wgpu/Vulkan), [mlua](https://github.com/mlua-rs/mlua) (Luau), [rbx-dom](https://github.com/rojo-rbx/rbx-dom), [Lune](https://github.com/lune-org/lune) como base de tipos e como oráculo de testes.

## Estado atual

Medido em uma máquina Arch Linux, GPU Intel UHD, Wayland.

**Funciona e foi testado**
- Cliente Rojo ao vivo: editar um arquivo chega ao DataModel em cerca de 66 ms (teste com `rojo serve` real).
- Luau: `Instance.new("Part")`, `Parent = workspace`, isolamento de erro de script, `task.wait` e `task.spawn` determinísticos.
- Janela 3D a ~60 FPS, câmera isométrica com zoom, clique direito para andar a 16 studs/s com marcador de destino.

![Jogo de exemplo rodando a partir de scripts Luau, com hot reload](studio/docs/img/demo.gif)

- As Parts criadas por um script Luau são desenhadas em 3D e acompanham o script a cada quadro. Um patch do Rojo reconstrói o mundo com o código novo (hot reload).

**Em andamento ou ainda não feito**
- `part.Position` ainda não é derivada de `CFrame` (lacuna da biblioteca em que nos apoiamos; veja `studio/docs/decisions.md`).
- Separar servidor e cliente: hoje todo Script e LocalScript roda no mesmo estado Luau.
- Física e colisão, `Humanoid`, `UserInputService` real, GUI, som.
- Efeitos visuais (aura, brilho, partículas): é o foco do jogo-alvo e vem depois do básico.
- Personagem: um boneco de blocos próprio ou um modelo de licença aberta (veja abaixo).


## Regras do projeto

- **Clean-room.** Só fontes públicas e abertas. Nada de descompilar nem copiar binários, modelos, malhas, texturas ou sons do Roblox, nem cópias deles encontradas na internet. Assets de terceiros só com licença aberta clara (CC0, CC-BY etc.), registrados com autor, link e licença.
- **Sem pânico no runtime.** Nos crates de execução do jogo, `unwrap`, `expect` e `panic` são bloqueados pelo clippy. Erro de script não derruba a janela.
- **Dado não confirmado vira "a confirmar".** Nada de número inventado.
- **Número vence opinião.** Cada comportamento esperado vira um teste (`studio/docs/resultado-esperado.md`).

## Como rodar

Requisitos (Arch): `sudo pacman -S --needed base-devel git clang cmake rustup python ffmpeg vulkan-intel vulkan-tools`, mais `rustup default stable`.

```bash
./setup.sh                      # clona as referências e instala lune e rojo
cd studio
cargo test --workspace          # testes
cargo run -p app                # abre a janela (a primeira compilação do Bevy demora)
```

O app tem um modo de teste por variáveis de ambiente, usado pelo QA: `STUDIO_CLICKS`, `STUDIO_SHOTS` (print só da janela) e `STUDIO_EXIT_AFTER` (imprime FPS e posição final).

O jogo-alvo (RUBIX) não faz parte deste repositório. Para testar, use o jogo de exemplo `studio/harness/sample-game`:

```bash
rojo serve harness/sample-game/default.project.json &
STUDIO_ROJO=http://127.0.0.1:34872 cargo run -p app
```

## Documentação

- [`studio/docs/decisions.md`](studio/docs/decisions.md), [`rojo-protocol.md`](studio/docs/rojo-protocol.md) e [`bug-prevention.md`](studio/docs/bug-prevention.md): em inglês.
- [`CONTRIBUTING.md`](CONTRIBUTING.md): como contribuir.

## Licença

Licença dupla: [MIT](LICENSE-MIT) ou [Apache-2.0](LICENSE-APACHE), à sua escolha. Rojo e Lune são MPL-2.0 e entram só como dependência ou binário, sem cópia de arquivos deles.

Este projeto não é afiliado, endossado nem patrocinado pela Roblox Corporation. "Roblox" é marca de seu titular.
