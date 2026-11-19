# Simulação

Este documento descreve o que a simulação faz **hoje** (M5 em andamento) e
como estendê-la.
As fórmulas atuais são provisórias: existem para provar o loop e dar
consequência às decisões, não para estarem balanceadas.

## Estruturas

- `World`: ano, mês, facções, províncias, RNG, fila de comandos e diplomacia.
- `Faction`: nome, tesouro, legitimidade (0–100), imposto (0–100), exército e
  custo de corte.
- `Province`: nome, dono (índice da facção), desenvolvimento, descontentamento
  (0–100) e populações.
- `Pops`: quatro classes — `peasant`, `burgher`, `noble`, `clergy`.
- `Diplomacy`: matriz de relações, matriz de tréguas e lista de guerras.
- `War`: atacante, defensor e data de início.
- `ModifierDef` / `ActiveModifier`: definição de modificador e instância ativa
  (com meses restantes ou permanente) em uma província ou facção.
- `EventDef`: evento com escopo, chance, cooldown, condições e efeitos;
  `event_log` guarda os últimos 200 eventos ocorridos.
- `PendingDecision`: escolha aguardando o jogador (`World.player`), com prazo.

Todas as quantias usam `i64`. Não há ponto flutuante na simulação.

## Comandos

Comandos são dados enfileirados no `World` e aplicados **no início do próximo
tick**, em ordem. Comandos inválidos são ignorados (facção/província
inexistente, valores não positivos, guerra duplicada, trégua ativa) ou têm
valores limitados (imposto é `clamp`ado em 0–100).

| Comando | Efeito |
| --- | --- |
| `SetTax { faction, rate }` | Ajusta o imposto da facção (0–100) |
| `Recruit { faction, soldiers }` | Gasta `soldiers * 2` de ouro e soma ao exército, se houver fundos |
| `Disband { faction, soldiers }` | Remove soldados do exército, sem ficar negativo |
| `DeclareWar { attacker, defender }` | Declara guerra se não houver guerra/trégua entre eles |
| `MakePeace { faction, other }` | Encerra a guerra e inicia trégua de 24 meses |
| `Attack { faction, province }` | Tenta tomar uma província inimiga em guerra |
| `SetPlayer { faction }` | Define a facção do jogador (`None` = todas com a IA) |
| `ChooseOption { faction, decision, option }` | Resolve uma decisão pendente da facção |

O jogador emite comandos pelo REPL (`crownveil-repl`). A UI futura emitirá os
mesmos comandos.

## Pipeline de um tick (`tick_month`)

1. **Aplicar comandos pendentes** (na ordem; ataques são resolvidos aqui).
2. **Avançar calendário.** `month += 1`; ao passar de 12, volta a 1 e
   `year += 1`.
3. **Para cada província**, com base no dono:
   1. calcular receita (imposto) e despesa e somar ao tesouro da facção;
   2. calcular o alvo de descontentamento e mover 10% da distância;
   3. crescer as populações respeitando a capacidade de alimento;
   4. aplicar fome se a população exceder a capacidade.
4. **Efeitos de guerra**: atrito militar, perda de legitimidade e relação −100.
5. **Cobrar custos de facção**: corte + manutenção do exército.
6. **Atualizar legitimidade** (convergência para 50 + penalidades).
7. **Tick da diplomacia**: contar tréguas e aproximar relações de 0 em paz.
8. **Choques aleatórios** de descontentamento.
9. **Modificadores**: decrementa a duração e remove os que chegaram a 0.
10. **Eventos**: primeiro resolve, com a primeira opção, as decisões cujo prazo
    venceu. Depois, para cada evento (em ordem) e cada alvo (em ordem), se não
    estiver em cooldown e todas as condições valerem, rola `rng(0..999) <
    chance`; se passar, aplica os efeitos, registra cooldown e então: sem
    opções, entra no log; com opções, a IA aplica a primeira opção na hora ou,
    se a facção que decide for a do jogador, cria uma decisão pendente.

## Economia e população

### Capacidade de alimento

```
capacidade = development * 400
densidade  = população_total * 100 / capacidade
```

`development = 50` ⇒ capacidade de 20.000 pessoas. A capacidade representa a
produção de comida agregada; escassez explícita (estoques, colheitas) fica para
depois.

### Receita (imposto)

```
base      = peasant/100 + burgher/50 + noble/20
potencial = base * development / 50
receita   = potencial * tax_rate / 100
```

### Despesa por província

```
despesa = clergy/100 + noble/100
```

### Custos de facção

```
manutencao_exercito = army / 10
custos = court_upkeep + manutencao_exercito
```

### Alvo de descontentamento

```
alvo = clamp(
    (100 - legitimidade)/2
  + (se tesouro < 0 { 20 } senão { 0 })
  + tax_rate/5
  + (se fome { 10 } senão se densidade > 90 { (densidade - 90)/2 } senão { 0 })
  + (se a facção está em guerra { 8 } senão { 0 })
  + modificadores.unrest (província + facção dona),
  0, 100
)
```

A cada mês: `descontentamento += (alvo - descontentamento)/10`, com `clamp`
em 0–100. `World::explain_unrest` retorna essa decomposição para o jogador e
para o REPL.

### Crescimento populacional

```
room     = (capacidade - total) * 100 / capacidade   // 0..100
pressao  = 100 - descontentamento
peasant += (peasant/100) * pressao/100 * room/100
se descontentamento > 80: peasant -= peasant/200
se tesouro > 0: burgher += (burgher/200) * room/100
noble  += (noble/400) * room/100
clergy += (clergy/500) * room/100
```

### Fome

Se `total > capacidade`:

```
excesso = total - capacidade
mortes  = clamp(excesso/20, 1, total)
mortes são distribuídas proporcionalmente entre as classes
descontentamento += 3
```

### Legitimidade

```
se legitimidade < 50: legitimidade += 1
se legitimidade > 50: legitimidade -= 1
se tesouro < -500 ou tax_rate > 50: legitimidade -= 1
legitimidade = clamp(legitimidade, 0, 100)
```

## Modificadores

Efeitos somam entre todos os modificadores ativos:

| Efeito | Onde vale | Regra |
| --- | --- | --- |
| `unrest` | província e facção (todas as suas províncias) | soma ao alvo de descontentamento |
| `income_pct` | província e facção | `receita = receita * (100 + pct) / 100` |
| `food_pct` | só província | `capacidade = capacidade * (100 + pct) / 100` |
| `legitimacy` | só facção | soma à legitimidade a cada mês |

Percentuais nunca invertem o sinal (`100 + pct` é limitado a 0). Reaplicar um
modificador ativo não acumula: renova a duração (fica a maior).

Um modificador adicionado no mês *T* vale nos meses *T+1* … *T+N* e some no
fim do mês *T+N*.

## Eventos

- **Escopo** `Province` (cada província) ou `Faction` (cada facção com ao
  menos uma província).
- **Condições** de província (`UnrestAtLeast`, `Famine`, `ClergyAtLeast`, …)
  só valem no escopo de província. Condições de facção (`LegitimacyBelow`,
  `AtWar`, …) olham a facção dona quando o escopo é província.
  `HasModifier`/`LacksModifier` olham o próprio alvo.
- **Efeitos**: `Unrest`, `Development` e `Population` (%) agem na província
  ou em todas as províncias da facção; `Legitimacy`, `Treasury` e `Army` agem
  na facção (dona, no escopo de província); `AddModifier`/`RemoveModifier`
  agem no próprio alvo.

### Escolhas

- A facção que decide é a dona da província (escopo `Province`) ou a própria
  facção (escopo `Faction`), fixada no momento do evento.
- `effects` do evento valem na hora; os `effects` da opção, quando decidida.
- O jogador tem 12 meses (`DECISION_DEADLINE_MONTHS`); depois vale a primeira
  opção. Escolhas inválidas (outra facção, opção inexistente) são ignoradas.
- O log registra o evento quando ele é resolvido, com `choice: Some(índice)`.

## Diplomacia

- Relações são simétricas, em −100..100, e começam em 0.
- Em paz, cada relação se aproxima de 0 em 1 ponto por mês.
- Declarar guerra exige relação sem guerra ativa e sem trégua; define a relação
  em −100.
- Paz remove a guerra, define a relação em −50 e cria trégua de 24 meses.
- Tréguas contam para baixo 1 mês por tick e bloqueiam nova declaração.
- Se uma facção perde a última província, todas as suas guerras terminam.

## Guerra

### Guarnição

```
guarnicao = development * 10 + população_total / 500
```

### Resolução de `Attack`

Se `exército_atacante > exército_defensor + guarnição`:

```
perdas_do_atacante = guarnicao / 5
província passa para o atacante
legitimidade do atacante += 3 (máx. 100)
legitimidade do defensor -= 5 (mín. 0)
descontentamento da província += 10
relação = -100
```

Caso contrário:

```
perdas_do_atacante = max(guarnicao / 10, 1)
descontentamento da província += 5
```

### Atrito mensal

Para cada guerra ativa, cada lado perde `max(army/100, 1)` soldados por mês e
1 de legitimidade. A relação permanece −100 enquanto durar a guerra.

### Choque aleatório

```
se rng(0..999) < 10:                       // 1% por província/mês
    descontentamento += rng(-5..=5)
    descontentamento = clamp(descontentamento, 0, 100)
```

Todo consumo de RNG passa por `sim::Rng`. A ordem de consumo importa e faz
parte do contrato de determinismo.

## Invariantes

- `legitimidade`, `descontentamento` e `tax_rate` ficam em 0–100.
- Relações ficam em −100..100; tréguas nunca são negativas.
- Populações e exército nunca são negativos.
- Não existem duas guerras entre o mesmo par de facções.
- `province.owner` sempre aponta para uma facção válida (garantido pelo loader).
- Mesma seed + mesma sequência de comandos/ticks = mesmo `World`.

## Pontos de extensão planejados

- **IA de facções**: declarar guerra, recrutar e atacar por conta própria.
- **Diplomacia profunda**: alianças, casamentos, vassalagem, casus belli.
- **Guerra no mapa**: exércitos em províncias, cercos, frentes e suprimento.
- **Negociação de paz**: aceitação baseada em desgaste, não comando imediato.
- **IA de escolhas**: avaliar opções em vez de sempre pegar a primeira.
- **Fé e magia**: recursos e modificadores que interagem com legitimidade e
  produção.
- **Comida explícita**: estoques, colheitas e rotas de suprimento.

Ao adicionar um sistema, mantenha a regra no crate `sim` e os dados em
`content/`. Registre decisões relevantes em `docs/06-adr.md`.
