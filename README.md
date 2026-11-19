# Crownveil

> Grand strategy de fantasia em tempo real com pausa. Reinos, fé, linhagens e
> guerra — com simulação determinística e conteúdo orientado a dados.

**Crownveil** ("Véu da Coroa") é um jogo de estratégia em que você governa uma
facção fantástica: administra províncias, populações e legitimidade, negocia e
guerra contra reinos rivais, enquanto eventos mágicos e religiosos moldam o
mundo. O tempo corre em tempo real, mas pode ser pausado e acelerado.

## Status

Protótipo jogável (**M4**, com o **M5** em andamento): simulação
determinística em ticks mensais com economia, comida/fome, diplomacia e guerra,
mais uma interface gráfica em `egui`/`eframe` com pausa, velocidades, painéis
de facção/província, comandos básicos e mapa hexagonal 2D com overlays,
seleção, tooltips, zoom e pan. Eventos e modificadores orientados a dados
(pestes, colheitas, revoltas, presságios) aparecem na Crônica da UI; vários
deles são dilemas em que o jogador escolhe a resposta. Fé e magia são recursos
das facções, gastos em ritos (curar a peste, conjurar chuva, selar o Véu…).

## Requisitos

- Rust stable (instale via [rustup](https://rustup.rs))

## Começando

```bash
# roda 100 anos de simulação com a seed padrão
cargo run -p crownveil-cli

# roda 50 anos com a seed 42
cargo run -p crownveil-cli -- 50 42

# modo interativo (REPL) para emitir comandos
cargo run -p crownveil-cli --bin crownveil-repl

# interface gráfica
cargo run -p crownveil-client

# testes (determinismo, comandos, economia, guerra e UI)
cargo test

# formatação e lint
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
```

No REPL, os comandos são: `status`, `provinces`, `tax`, `recruit`, `disband`,
`war`, `peace`, `attack`, `wars`, `events`, `decisions`, `choose`, `player`,
`rites`, `rite`, `step`, `explain`, `help` e `quit`. O REPL joga com a facção 0; `step` para
quando surge uma decisão, e `choose <decisão> <opção>` responde. Use
`step N` para aplicar os comandos enfileirados e avançar `N` meses.

Na UI: **Continuar/Pausar**, **+1 mês**, velocidades 1x–8x, seleção de facção
jogável e de província, além de botões para imposto, recrutamento, guerra, paz
e ataque. No mapa: clique para selecionar, arraste para mover, use o scroll para
zoom e troque a camada entre **Dono**, **Descontentamento** e **População**.
Quando um evento exige uma escolha da sua facção, o jogo pausa e abre uma
janela de decisão; passe o mouse sobre as opções para ver os efeitos. A seção
**Ritos** (painel da direita) lista os ritos com custo; ritos locais agem na
província selecionada.

## Estrutura do workspace

| Crate | Papel |
| --- | --- |
| `crates/sim` | Núcleo determinístico da simulação. Não conhece UI nem conteúdo. |
| `crates/content` | Carrega conteúdo (RON) e monta um `World` da simulação. |
| `crates/cli` | Composition root headless: roda a simulação e imprime relatórios. |
| `crates/client` | Cliente gráfico em `egui`/`eframe`: lê o `World` e emite comandos. |
| `content/` | Dados do jogo: facções, províncias, modificadores, eventos e ritos. |
| `docs/` | Documentação de visão, arquitetura, design e roadmap. |

## Documentação

- [Visão do jogo](docs/00-vision.md)
- [Arquitetura](docs/01-architecture.md)
- [Simulação](docs/02-simulation.md)
- [Pipeline de conteúdo](docs/03-content-pipeline.md)
- [Roadmap](docs/04-roadmap.md)
- [Padrões de código](docs/05-coding-standards.md)
- [Registro de decisões (ADR)](docs/06-adr.md)
- [Glossário](docs/07-glossary.md)
- [M4 — Mapa 2D](docs/08-m4-mapa.md)

## Licença

MIT OR Apache-2.0
