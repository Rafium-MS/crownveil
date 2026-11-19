# Roadmap

Marcos pequenos e verificáveis. Não avance um marco sem o anterior estar
divertido (ou pelo menos testado) headless.

## M0 — Fundação (concluído)

- [x] Workspace Cargo com `sim`, `content`, `cli`, `client`
- [x] Tick mensal determinístico
- [x] Populações, tesouro, legitimidade e descontentamento
- [x] Conteúdo inicial em RON + loader
- [x] Runner headless com relatórios
- [x] Testes de determinismo
- [x] Documentação inicial

## M1 — Comandos e economia jogável (concluído)

- [x] Tipo `Command` e fila de comandos por tick
- [x] Impostos por facção (afetam receita e descontentamento)
- [x] Custos reais de exército e corte (fé fica para M5)
- [x] Capacidade de alimento, escassez e fome
- [x] Relatório de causas (`explain_unrest`)
- [x] Testes de comandos, economia e invariantes
- [x] REPL no terminal para emitir comandos

## M2 — Diplomacia e guerra (concluído)

- [x] Relações entre facções (−100..100), com convergência em paz
- [x] Declaração de guerra, trégua e paz
- [x] Exércitos: recrutamento, manutenção e atrito mensal
- [x] Ataque e ocupação de províncias com guarnição
- [x] Guerra afetando legitimidade, descontentamento e exército
- [x] Comandos `DeclareWar`, `MakePeace` e `Attack` no REPL
- [x] Testes de diplomacia, guerra e determinismo

## M3 — Primeira interface (concluído)

- [x] `egui`/`eframe` no crate `client`
- [x] Pausa, play, `+1 mês` e velocidades (1x/2x/4x/8x)
- [x] Painel de facções: tesouro, legitimidade, imposto, exército, províncias
- [x] Lista de províncias com população, capacidade e descontentamento
- [x] Detalhes da província com decomposição do descontentamento
- [x] Comandos básicos pela UI (imposto, recrutar/dispensar, guerra/paz, ataque)
- [x] Testes do relógio de ticks e smoke test da janela

## M4 — Mapa (concluído)

- [x] Renderização 2D de províncias em grade hexagonal
- [x] Seleção de província e tooltips
- [x] Overlays: descontentamento, população e dono
- [x] Zoom (scroll/botões) e pan (arrastar)
- [x] Posições de mapa definidas no conteúdo (`map: Some((col, row))`)
- [x] Testes de geometria/hit-testing e smoke test com screenshot

## M5 — Fantasia e eventos (concluído)

- [x] Sistema de eventos orientado a dados (`content/events.ron`)
- [x] Modificadores temporários e permanentes em províncias e facções
- [x] Crônica de eventos na UI, comando `events` no REPL e log na CLI
- [x] Primeiros presságios e crises (cometa, crise de sucessão, Véu rasgado)
- [x] Fé/magia como recursos, modificadores e ritos (`content/rites.ron`)
- [x] Eventos com escolhas do jogador (decisões pendentes, IA e prazo)
- [x] Crises encadeadas com consequências de longo prazo (`TriggerEvent`)

## M6 — Persistência e polimento

- [ ] Save/load versionado
- [ ] Migração de saves
- [ ] Balanceamento assistido por simulação em lote
- [ ] Acessibilidade, atalhos e configurações
- [ ] Builds de release para Linux (e depois outras plataformas)

## Fora de escopo por enquanto

Multiplayer, 3D, batalhas táticas, editor de mapas no jogo, dinastias
detalhadas. Reavalie apenas depois de M6.
