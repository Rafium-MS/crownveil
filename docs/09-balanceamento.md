# Balanceamento em lote

`crownveil-batch` roda muitas partidas headless (todas as facções com a IA) e
resume o que aconteceu. Use antes e depois de mexer em números de conteúdo ou
de regra.

```bash
cargo run --release -p crownveil-cli --bin crownveil-batch -- --runs 200 --years 100
cargo run --release -p crownveil-cli --bin crownveil-batch -- --runs 50 --seed 1000 --csv lote.csv
```

| Opção | Padrão | Significado |
| --- | --- | --- |
| `--runs N` | 100 | número de partidas (seeds consecutivas) |
| `--years N` | 100 | anos por partida |
| `--seed N` | 1 | primeira seed |
| `--csv arquivo` | — | grava uma linha por partida (para planilha) |

As partidas rodam em paralelo, mas o relatório é idêntico com qualquer número
de núcleos: cada seed é independente e o resultado é ordenado por seed.

## O que o relatório mostra

- **Mundo**: população final, descontentamento médio e de pico (média, p10,
  mediana, p90) e a fração de províncias-mês em fome ou com descontentamento
  ≥ 70.
- **Facções**: tesouro, legitimidade e províncias no fim; fé e magia médias ao
  longo da partida.
- **Eventos**: média por partida, frequência por alvo por século (divide pelo
  número de províncias ou facções) e em quantas partidas o evento apareceu.
- **Modificadores**: quantos alvo-meses cada um ficou ativo.
- **Alertas**: eventos que nunca acontecem, eventos acima de 4 vezes por alvo
  por século e fome acima de 15% das províncias-mês.

Limitação: sem jogador e sem IA de guerra ou de ritos, eventos que dependem de
guerra, de legitimidade extrema ou de escolhas do jogador não aparecem. Os
alertas desses eventos são esperados até existir IA de facção.

## Rodada de 2026-11-19 (200 partidas × 100 anos)

| Evento | Antes (por partida) | Depois | Mudança |
| --- | --- | --- | --- |
| Colheita Farta | 50,5 (5,6/alvo/século) | 27,6 (3,1) | `chance` 6 → 3 |
| Profeta Errante | 14,2 | 8,7 | `chance` 4 → 2 |
| O Véu se Rasga | 8,9 | 5,2 | nova condição `ManaAtLeast(250)` |
| Peste Cinzenta / Segunda Onda | 13,6 / 13,5 | 11,8 / 11,7 | efeito indireto |

Fome ficou em 3,4% das províncias-mês e descontentamento ≥ 70 em 0,8%.

O limiar de magia do Véu tem um degrau: com 250 o evento ocorre ~5 vezes por
partida; com 270, nunca, porque a magia das facções se estabiliza perto de 250
(geração × 50, ver ADR-0012). Deixar o Véu realmente raro pede chance mais fina
que 1‰ por mês.

## Problemas em aberto (decisão de design)

- **Tesouro sem freio**: as facções terminam o século com 80–170 mil de ouro
  (começam com ~1.000). Sem IA que gaste, o ouro não tem destino; mesmo para o
  jogador, ele perde valor depois de alguns anos.
- **Legitimidade presa em 50**: a convergência de ±1 por mês domina; por isso
  Crise de Sucessão (< 35) e Era Dourada (≥ 65) quase nunca acontecem sem o
  jogador.
- **Granularidade da chance**: `CHANCE_SCALE = 1000` impede eventos mais raros
  que 1,2 vez por alvo por século.
