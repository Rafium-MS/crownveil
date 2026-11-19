# Registro de decisões de arquitetura (ADR)

Formato curto: contexto, decisão, consequências.

## ADR-0001 — Rust com workspace multi-crate

**Contexto:** o projeto crescerá em regras de simulação, conteúdo e UI.

**Decisão:** usar um workspace Cargo com `sim`, `content`, `cli` e `client`.

**Consequências:** fronteiras claras; `sim` permanece testável e leve; builds
incrementais mais rápidos; alguma cerimônia extra para mover tipos entre crates.

## ADR-0002 — Simulação determinística separada da apresentação

**Contexto:** grand strategy precisa de reprodutibilidade para testes, saves e
balanceamento; UI muda com frequência.

**Decisão:** toda regra vive em `sim`, que não conhece UI nem I/O. O cliente lê
o estado e envia comandos.

**Consequências:** simulação roda headless; testes baratos; a UI não pode
"colar" regra de jogo. Requer disciplina de comandos em vez de mutação direta.

## ADR-0003 — Inteiros em vez de ponto flutuante

**Contexto:** floats variam entre plataformas e quebram determinismo.

**Decisão:** usar `i64` (unidades inteiras ou fixed-point explícito) em `sim`.

**Consequências:** reprodutibilidade e saves estáveis; fórmulas precisam de
cuidado com arredondamento.

## ADR-0004 — Conteúdo em RON

**Contexto:** precisamos de dados legíveis e com suporte a comentários.

**Decisão:** usar RON para conteúdo, com structs `serde` no crate `content`.

**Consequências:** bom encaixe com Rust e tipos; fácil de editar à mão. Se o
conteúdo crescer muito, avaliar ferramentas de autoria/validação.

## ADR-0005 — Headless primeiro

**Contexto:** o risco do projeto é a simulação, não o desenho da tela.

**Decisão:** validar economia, política e guerra sem gráficos antes de escolher
engine de renderização.

**Consequências:** feedback mais rápido; evita retrabalho de UI; pode parecer
"lento" para quem quer ver o jogo na tela.

## ADR-0006 — Tempo real com pausa como ticks fixos

**Contexto:** queremos tempo real com pausa, mas determinismo.

**Decisão:** o tempo real é apenas uma taxa de execução de ticks fixos
(1 tick = 1 mês). Pausa = 0 ticks/s.

**Consequências:** save/load e replay triviais; velocidade não afeta a
simulação; a UI precisa acumular tempo real e executar ticks inteiros.

## ADR-0007 — ECS não é obrigatório

**Contexto:** Bevy sugere ECS, mas grand strategy é fortemente baseado em
grafo/índices (províncias, facções, rotas).

**Decisão:** usar structs e índices (`Vec`, `slotmap` quando necessário) em
`sim`. ECS só se provar útil na camada de apresentação.

**Consequências:** simulação mais simples de raciocinar e serializar; menos
"magia" de queries; abrimos mão de paralelismo automático de ECS (que também
poderia ameaçar determinismo).

## ADR-0008 — `egui`/`eframe` para a primeira interface

**Contexto:** o M3 precisa de uma UI funcional para pausa, painéis e comandos,
sem investir ainda em mapa ou engine 3D.

**Decisão:** usar `egui` via `eframe` no crate `client`. A UI lê o `World`,
acumula tempo real e emite comandos; a simulação continua headless e
determinística.

**Consequências:** protótipo rápido e integrado ao Rust; dependência pesada de
build no cliente (não afeta `sim`); o mapa 2D futuro pode reutilizar `egui` ou
migrar para outra engine sem tocar nas regras. Se a UI crescer, avaliar separar
estado de apresentação em módulos/crates.

## ADR-0009 — Mapa em grade hexagonal com posições no conteúdo

**Contexto:** o M4 precisa de um mapa 2D selecionável sem exigir geometria
geográfica complexa nem editor de polígonos.

**Decisão:** representar cada província como um hexágono em coordenadas
*offset* (`col`, `row`) definidas no RON. O `content` expõe um `MapLayout`; o
cliente converte para tela, desenha e faz hit-testing por centro mais próximo.

**Consequências:** mapa legível e barato de implementar; posições são dados
editáveis; sem adjacências ou formas irregulares por enquanto. Migrar para
polígonos geográficos depois não muda as regras da simulação.

## ADR-0010 — Eventos e modificadores como dados

**Contexto:** o M5 exige pestes, presságios e crises sem codificar cada um à
mão, mantendo determinismo e legibilidade das causas.

**Decisão:** `sim` define um vocabulário fechado de condições (`Condition`) e
efeitos (`Effect`) e um tipo `ModifierEffects` aditivo. O conteúdo descreve
eventos e modificadores em RON, referenciando modificadores por chave; o
`content` valida e converte para índices. Eventos são avaliados no fim do tick,
em ordem de índice, e só consomem RNG quando o alvo passa nas condições e no
cooldown. Um modificador ativo copia os efeitos da definição para que as regras
funcionem a partir da província/facção isoladamente.

**Consequências:** novos eventos não exigem código, desde que caibam no
vocabulário; ampliar o vocabulário exige mudar `sim` e o espelho em `content`.
A decomposição de descontentamento ganha a parcela `modificadores`. Mudar a
ordem ou a chance dos eventos altera as partidas de uma seed, mas não o
determinismo.

## ADR-0011 — Jogador como parte do estado e escolhas por comando

**Contexto:** eventos com escolhas precisam esperar o jogador, enquanto as
outras facções decidem sozinhas. Quem é o jogador muda o resultado da partida.

**Decisão:** `World.player: Option<FactionId>` faz parte do estado e só muda
por `Command::SetPlayer`. Quando um evento com opções atinge a facção do
jogador, vira uma `PendingDecision`, resolvida por `Command::ChooseOption` no
início do próximo tick. Facções da IA escolhem a primeira opção na hora; se o
jogador não responder em 12 meses, a primeira opção também é aplicada.

**Consequências:** replays continuam determinísticos (a escolha é um comando);
o autor de conteúdo controla o comportamento da IA pela ordem das opções; a
escolha só faz efeito no mês seguinte, então a UI esconde decisões já
respondidas. Uma IA que avalie as opções fica para depois e pode substituir a
regra "primeira opção" sem mudar o formato de conteúdo.

## ADR-0012 — Fé e magia como recursos com perda, gastos em ritos

**Contexto:** a visão pede fé e magia integradas às variáveis existentes, sem
mini-jogos. Sem IA que gaste, recursos acumuláveis lotavam o teto em poucos
anos e perdiam sentido.

**Decisão:** fé e magia são inteiros por facção (0–1000), gerados pelas
províncias (clero e desenvolvimento) e por modificadores, com perda de 2% do
estoque ao mês. Fé reduz e magia aumenta o alvo de descontentamento (até 5
pontos cada). O gasto acontece em ritos definidos em `content/rites.ron`, com o
mesmo vocabulário de condições e efeitos dos eventos e executados por
`Command::PerformRite`. `sim` expõe `check_rite`/`RiteBlock` para a UI explicar
indisponibilidade; o texto em português fica em `content::rite_block_text`.

**Consequências:** facções se diferenciam pelo perfil (Orem rica em fé, reinos
desenvolvidos com mais magia) e o jogador tem motivo para gastar. Ritos novos
não exigem código. A IA ainda não realiza ritos; quando houver IA de facção, ela
usa os mesmos comandos.

## ADR-0013 — Crises encadeadas por agendamento condicionado

**Contexto:** o M5 pede crises com consequências de longo prazo. Escrever cada
crise como código, ou como uma máquina de estados especial, iria contra
"dados antes de código".

**Decisão:** um único efeito, `TriggerEvent(evento, meses)`, agenda uma
continuação no mesmo alvo (`World.scheduled_events`). Ao vencer, ela ignora a
chance, mas precisa que as próprias condições ainda valham; se não valerem, é
descartada. A continuação de facção a partir de província vai para a dona. Cada
agendamento guarda o evento de origem, para a UI avisar ("consequência de X em
N meses") sem revelar o desfecho.

**Consequências:** crises de vários elos ficam inteiramente em RON, com
ramificações pelas opções; o jogador pode evitar um elo mudando o estado do
jogo (baixar descontentamento, recuperar legitimidade). Ciclos são possíveis,
mas avançam no máximo um elo por mês. A ordem das opções define o
comportamento da IA, então o autor precisa escolher se a IA escala cada crise.

## ADR-0014 — Saves versionados em RON com o mundo completo

**Contexto:** o M6 pede save/load. O `World` já é serializável e determinístico;
falta definir formato, versão e o que entra no arquivo.

**Decisão:** o save é um RON `(format_version, game_version, world)` com o
`World` completo, incluindo as definições de conteúdo. A serialização mora em
`content::save` (que já depende de `ron`), mantendo `sim` sem I/O. O loader lê
primeiro só a versão: atual carrega, maior é recusada, menor fica para
migrações explícitas. A escrita é atômica (arquivo temporário + `rename`).

**Consequências:** saves são legíveis e depuráveis à mão, e continuam válidos
mesmo que o conteúdo embutido mude; o custo é tamanho (dezenas de KB) e
duplicação das definições em cada save. Mudanças no formato de `World` que não
sejam aditivas com `#[serde(default)]` exigem subir `SAVE_FORMAT_VERSION` e
escrever a migração correspondente.
