# M4 — Mapa 2D

**Status:** concluído.

## Entregas

- Mapa hexagonal 2D renderizado no cliente `egui`/`eframe`.
- Cada província tem posição `col`/`row` definida no conteúdo:
  `map: Some((col: 0, row: 0))` em `content/world.ron`.
- `content` expõe `MapLayout`, carregado por `load_world_with_map`.
- Seleção de província por clique, com borda destacada.
- Tooltip com nome, dono, população/capacidade, desenvolvimento e
  descontentamento.
- Overlays: **Dono** (cor da facção), **Descontentamento**
  (verde → amarelo → vermelho) e **População** (azul).
- Zoom com scroll do mouse ou botões `+`/`−`; pan arrastando o mapa.
- Botão **Centrar** reenquadra o mapa na área disponível.
- Painel direito com detalhes da província e ação de ataque quando em guerra.

## Como usar

```bash
cd /media/rafium-mint/JOGOS/crownveil
cargo run -p crownveil-client
```

No mapa:

- **clique** seleciona a província;
- **arraste** move o mapa (pan);
- **scroll** aplica zoom;
- **Camada** troca entre Dono, Descontentamento e População;
- **Centrar** reenquadra;
- **+1 mês**, **Continuar/Pausar** e **1x–8x** controlam o tempo.

## Verificação

- `cargo fmt --all` e `cargo clippy --workspace --all-targets -- -D warnings`
  limpos.
- **30 testes** passando, incluindo geometria hexagonal, hit-testing,
  carregamento do `MapLayout` e relógio de ticks.
- Smoke test abriu a janela sem panic; a screenshot confirmou a presença das
  três cores de facção no mapa.

## Limitações

- Grade hexagonal simples; ainda não há polígonos geográficos.
- Sem adjacências, rotas ou exércitos representados no mapa.
- Layout definido manualmente no RON; um editor de mapa fica para depois.

## Próximo marco

**M5 — Fantasia e eventos:** sistema de eventos orientado a dados,
modificadores temporários e fé/magia como recursos e modificadores.
