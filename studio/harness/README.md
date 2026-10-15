# Harness de otimização

Ideia: medir, mudar uma coisa, medir de novo. Nada de "acho que melhorou".

| Script | Para quê | Quando rodar |
|---|---|---|
| `check.sh` | fmt, clippy e testes | a cada tarefa |
| `bench.py` | tempo de build, FPS (se houver binário `bench`) e regressão vs `baseline.json` | depois de mexer em render ou física |
| `compat_report.py` | quanto de um jogo (pasta de scripts) a engine suporta | a cada classe ou serviço novo |
| `visual_diff.py` | diferença entre captura e referência | quando houver print de referência |
| `diff_luau.py` | teste diferencial de scripts Luau contra o Lune (oráculo), com seed | a cada mudança na ponte Luau ou no scheduler |
| `fixtures/rojo/` | respostas reais do Rojo (bruta e decodificada) | criadas com a skill studio-protocol-capture |
| `evals/skills_evals.json` | prompts para testar e melhorar as skills | quando uma skill errar |
| `supported_api.json` | status de cada serviço e classe (`ok`, `planned`, `stub`, `missing`) | só marcar `ok` depois de teste passar |

## Ciclo de otimização do código
1. `python3 harness/bench.py --set-baseline` quando a medição estiver estável.
2. Faça uma mudança de cada vez.
3. `check.sh` e `bench.py`. Regressão acima de 10% volta para o dono.

## Ciclo de otimização das skills (no chat, sem subagentes)
1. Rode um prompt de `evals/skills_evals.json` com a skill.
2. Confira as verificações uma a uma.
3. Se falhou, ajuste o `SKILL.md` explicando o porquê da regra (regra sem motivo é ignorada) e rode de novo.
4. Acrescente ao arquivo de evals o caso que falhou na vida real.

## Limites do ambiente em nuvem
Sem tela: janela, FPS real e Wayland/X11 só na máquina do usuário (Arch, Intel, Vulkan). O `bench.py` só mede FPS se existir um binário `bench` que rode sem janela.
