# Padrões de código

## Linguagem

- Código, tipos, funções e nomes de API em **inglês**.
- Documentação, comentários de design e conteúdo do jogo em **português**.
- Comentários só quando explicam o *porquê*, nunca o *o quê*.

## Rust

- `cargo fmt --all` antes de qualquer entrega.
- `cargo clippy --workspace --all-targets -- -D warnings` deve passar.
- Prefira tipos explícitos e funções pequenas.
- Evite `unwrap`/`expect` em código de produção; propague erros com `Result`.
  Em testes, `unwrap` é aceitável.
- Sem `unsafe` sem uma ADR justificando.

## Determinismo

- Proibido `f32`/`f64` no crate `sim`.
- Aleatoriedade só via `sim::Rng` com seed explícita.
- Nada de `HashMap` decidindo ordem de simulação; use `Vec`, índices ou
  `BTreeMap`.
- Não use tempo de sistema, `thread_rng`, hash de ponteiro ou paralelismo não
  determinístico dentro de `sim`.

## Dados e números

- Recursos e populações em `i64`.
- Divisões são inteiras; documente o arredondamento quando importar.
- Faixas (0–100 etc.) devem ser `clamp`adas e cobertas por testes.

## Testes

- Todo sistema novo em `sim` precisa de teste.
- Testes de determinismo: mesma seed + mesmos ticks = mesmo estado.
- Testes de invariantes: populações ≥ 0, faixas válidas, donos válidos.
- Testes de conteúdo: o arquivo `content/world.ron` precisa carregar.

## Dependências

- Novas dependências exigem justificativa em `docs/06-adr.md`.
- `sim` deve permanecer leve: só `serde` (e olhe lá).
- Dependências de UI/engine ficam restritas a `client`.

## Commits (quando aplicável)

- Mensagens curtas no imperativo, em português ou inglês consistente.
- Um commit por mudança lógica; não misture refatoração com feature.
- Nunca commitar segredos, saves grandes ou `target/`.
