# Visão do jogo

## Pitch

**Crownveil** é um grand strategy de fantasia em tempo real com pausa. Você
comanda uma facção — um reino, uma corte, um domínio religioso — e precisa
equilibrar economia, populações, fé, legitimidade e guerra para sobreviver e
prosperar. A fantasia não é um sistema à parte: magia, raças e intervenções
divinas entram como **modificadores** sobre a mesma economia e política.

## Pilares de design

1. **Simulação legível.** O jogador consegue entender *por que* sua província
   está com fome ou por que a legitimidade caiu. Nada de caixas-pretas.
2. **Consequência lenta.** Decisões têm custo e retorno ao longo de anos, não
   em um turno. Guerras drenam tesouro, fé e população.
3. **Fantasia integrada.** Magia e religião alteram as mesmas variáveis do
   mundo (produção, descontentamento, legitimidade), sem mini-jogos isolados.
4. **Determinismo.** O mesmo save + as mesmas decisões = o mesmo resultado.
   Isso permite replay, depuração e balanceamento confiáveis.
5. **Dados antes de código.** Províncias, facções e eventos vivem em arquivos
   de conteúdo; adicionar conteúdo não deve exigir recompilar regras.

## Experiência alvo

O jogador passa a maior parte do tempo:

- observando o mapa e os indicadores do reino;
- pausando para tomar decisões (impostos, guerra, alianças, éditos);
- reagindo a eventos (pestes, revoltas, presságios, crises de sucessão);
- acelerando o tempo para ver o resultado das próprias escolhas.

## Inspirações

| Jogo | O que absorvemos |
| --- | --- |
| Victoria | populações, economia e industrialização como motor da simulação |
| Crusader Kings | política, legitimidade, intriga e consequências pessoais |
| Suzerain | peso narrativo das decisões e clareza das consequências |
| Europa Universalis | mapa, guerra e diplomacia em tempo real com pausa |

## Escopo (o que é e o que não é)

**É:**

- tempo real com pausa e velocidade ajustável;
- mapa de províncias, facções e populações;
- economia, descontentamento, legitimidade, diplomacia e guerra;
- eventos e modificadores de fantasia;
- single-player.

**Não é (por enquanto):**

- multiplayer;
- 3D ou batalhas táticas em tempo real;
- simulador de personagem individual (dinastias detalhadas vêm depois);
- editor de mapas completo no jogo.

## Público

Fãs de grand strategy que querem uma porta de entrada mais enxuta e com sabor
de fantasia, e fãs de RPG político que querem ver suas escolhas moldarem um
mundo simulado de verdade.
