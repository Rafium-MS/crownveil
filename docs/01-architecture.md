# Arquitetura

## Princípio central

**Simulação determinística separada da apresentação.** O crate `sim` roda sem
saber que existe uma tela. O cliente gráfico apenas lê o estado do mundo e
envia comandos. Isso permite rodar a simulação headless (em testes, em CI e em
ferramentas de balanceamento) e garante reprodutibilidade.

## Grafo de dependências

```
sim      -> (sem dependências internas; lógica pura e determinística)
content  -> sim
cli      -> sim, content
client   -> sim, content, eframe/egui
```

- `sim`: tipos do mundo e regras. Sem I/O, sem UI, sem dependências pesadas.
- `content`: desserializa arquivos RON e monta um `World`; também grava e lê
  saves versionados (`content::save`). É a fronteira de dados e serialização;
  `sim` continua sem I/O.
- `cli`: roda a simulação headless e imprime relatórios.
- `client`: cliente gráfico em `egui`/`eframe` (M3). Lê o `World` e emite
  comandos; não contém regra de jogo.

## Modelo de tempo

O tempo real com pausa é implementado como **ticks fixos**:

- 1 tick = 1 mês de jogo;
- 12 ticks = 1 ano;
- pausa = zero ticks por segundo;
- velocidades = N ticks por segundo (ex.: 1x, 2x, 4x, 8x).

O loop do cliente acumula tempo real e executa ticks inteiros. A simulação em
si nunca vê relógio, `Instant` ou `f64` — só recebe `tick_month()`.

```
loop do cliente:
    acumulador += delta_real * velocidade
    enquanto acumulador >= duracao_do_tick:
        sim.tick_month()
        acumulador -= duracao_do_tick
    desenhar(sim)
```

## Fluxo de dados

1. `content` lê RON e produz um `World`.
2. O jogador (via `cli` ou `client`) envia **comandos** (ex.: mudar imposto,
   declarar guerra). Comandos são dados, não chamadas diretas à UI.
3. A cada tick, `sim` aplica os comandos pendentes e avança as regras.
4. O cliente lê o `World` e desenha. Ele nunca muta o mundo diretamente.

> Hoje `World::queue_command` guarda os comandos e `tick_month` os aplica no
> início do tick. O REPL (`crownveil-repl`) e a UI (`crownveil-client`) emitem
> os mesmos comandos: `SetTax`, `Recruit`, `Disband`, `DeclareWar`, `MakePeace`
> e `Attack`.

## Determinismo

Regras:

- nenhum `f32`/`f64` em `sim` (use inteiros);
- RNG explícito e com seed (`sim::Rng`);
- iteração em ordem estável (`Vec`/índices; `BTreeMap` quando necessário);
- sem dependência de tempo de sistema, endereço de memória ou ordem de
  `HashMap`.

Consequências positivas: saves reproduzíveis, testes de regressão baratos,
replay de partidas e balanceamento automatizado.

## Save/load (planejado)

O `World` já deriva `Serialize`/`Deserialize`. O plano:

- salvar o `World` completo + versão do formato + seed;
- validar versão ao carregar e aplicar migrações;
- nunca salvar estado derivado que possa ser recalculado.

## Interface gráfica

O `client` usa `egui`/`eframe`: pausa/play, velocidades, painel de facções,
detalhes da província com decomposição do descontentamento, comandos básicos e
um mapa hexagonal 2D. O mapa tem seleção, tooltips, overlays (dono,
descontentamento, população), zoom e pan. As posições vêm do conteúdo
(`MapLayout`); o cliente só desenha e faz hit-testing.

A UI acumula tempo real e chama `tick_month`; nunca muta o `World` diretamente.

Próximos passos de apresentação:

1. polígonos geográficos em vez de grade hexagonal;
2. adjacências, rotas e animações de exércitos;
3. gráficos históricos e ferramentas de depuração;
4. avaliar engine dedicada se o mapa crescer muito.

## Estrutura de pastas

```
crates/
  sim/src/        lib.rs, world.rs, tick.rs, rng.rs, rules.rs, command.rs,
                  diplomacy.rs, event.rs, modifier.rs, rite.rs
  content/src/    lib.rs (loader RON), save.rs (saves versionados),
                  save/doc.rs (árvore RON sem perdas), save/migrations.rs
  content/tests/  fixtures/save_v<N>.ron (saves congelados por versão)
  cli/src/        main.rs (runner headless), bin/repl.rs
  client/src/     main.rs, app.rs (UI egui/eframe)
content/          world.ron, events.ron, rites.ron
docs/             documentação do projeto
```
