# AGENTS.md — Crownveil

Guia para agentes de código (e humanos) que trabalham neste repositório.

## Comandos

```bash
cargo run -p crownveil-cli                     # 100 anos, seed 1
cargo run -p crownveil-cli -- 50 42            # 50 anos, seed 42
cargo run -p crownveil-cli --bin crownveil-repl  # modo interativo (comandos)
cargo run -p crownveil-client                  # interface gráfica (egui)
cargo run --release -p crownveil-cli --bin crownveil-batch -- --runs 200  # balanceamento em lote
cargo test                                     # testes do workspace
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
```

Sempre rode `cargo fmt --all`, `cargo clippy ...` e `cargo test` antes de
considerar uma tarefa concluída.

## Arquitetura (regras de dependência)

```
sim  <-  content  <-  cli
 ^                    ^
 +-------------------- client
```

- `sim` **não pode** depender de `content`, `cli`, `client`, Bevy, `egui` ou de
  qualquer crate de I/O/UI. É lógica pura.
- `content` depende de `sim` e converte dados em tipos da simulação.
- `cli` e `client` são *composition roots*: montam o mundo e orquestram.
- Nunca coloque regra de jogo em `content` ou `cli`; regra de jogo vive em `sim`.

## Regras de determinismo (inegociáveis)

1. **Sem floats** (`f32`/`f64`) no crate `sim`. Use inteiros (unidades inteiras
   ou fixed-point explícito).
2. **Toda aleatoriedade** passa por `sim::Rng` (seed explícita). Nunca use
   `rand::thread_rng`, tempo de sistema, hash de endereço ou ordem de `HashMap`
   para decidir algo no jogo.
3. **Itere em ordem estável**: `Vec`/índices. Se precisar de mapa, use
   `BTreeMap` ou ordene antes.
4. Um mundo com a mesma seed e as mesmas entradas deve produzir exatamente o
   mesmo estado. Há um teste garantindo isso em `crates/sim/tests/`.

## Onde colocar as coisas

- Nova regra/economia: `crates/sim/src/rules.rs` + `tick.rs`.
- Diplomacia e guerra: `crates/sim/src/diplomacy.rs` + `tick.rs`.
- Novo comando do jogador: `crates/sim/src/command.rs` + tratamento em
  `tick.rs` + REPL em `crates/cli/src/bin/repl.rs`.
- Novo tipo de dado de conteúdo: `crates/content/src/lib.rs` + arquivo em
  `content/`.
- Novo comando/report: `crates/cli/src/main.rs`.
- Mudou números de conteúdo ou de regra: rode `crownveil-batch` antes e depois
  e registre achados relevantes em `docs/09-balanceamento.md`.
- Mudança de UI: `crates/client/src/app.rs` (estado/relógio) e `main.rs`
  (janela). Regra de jogo continua em `sim`.
- Mudança em tipos salvos (`World` e o que ele contém): se não couber em um
  campo novo com `#[serde(default)]`, suba `SAVE_FORMAT_VERSION`, escreva a
  migração em `crates/content/src/save/migrations.rs` e adicione
  `crates/content/tests/fixtures/save_v<N>.ron`. Nunca altere fixtures antigas.
- Decisão de arquitetura relevante: registre em `docs/06-adr.md`.
- Mudança de escopo/visão: atualize `docs/00-vision.md` e `docs/04-roadmap.md`.

## Estilo

- Código e nomes de API em inglês; docs e conteúdo do jogo em português.
- Sem comentários explicando o óbvio; comente apenas o "porquê".
- Prefira funções pequenas e tipos explícitos (`i64` para recursos).
- Não adicione dependências pesadas sem registrar uma ADR.
