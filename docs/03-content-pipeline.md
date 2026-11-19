# Pipeline de conteúdo

Conteúdo do jogo vive em arquivos de dados (RON) e é carregado pelo crate
`crownveil-content`. Regras ficam em `crownveil-sim`; dados ficam em `content/`.

## Arquivo atual

`content/world.ron` define facções e províncias do cenário inicial.

```ron
(
    factions: [
        (name: "Reino de Aeldrath", treasury: 1000, legitimacy: 60),
    ],
    provinces: [
        (
            name: "Pedra Alta",
            owner: "Reino de Aeldrath",
            map: Some((col: 0, row: 0)),
            development: 55,
            unrest: 20,
            peasant: 12000,
            burgher: 2500,
            noble: 300,
            clergy: 400,
        ),
    ],
)
```

### Campos

**Facção**

| Campo | Tipo | Descrição |
| --- | --- | --- |
| `name` | string | Nome único; usado como referência de dono |
| `treasury` | inteiro | Tesouro inicial |
| `legitimacy` | inteiro | Legitimidade inicial (0–100) |
| `tax_rate` | inteiro | Imposto inicial (0–100); padrão 20 |
| `faith` | inteiro | Fé inicial (0–1000); padrão 0 |
| `mana` | inteiro | Magia inicial (0–1000); padrão 0 |
| `army` | inteiro | Soldados iniciais; padrão 0 |
| `court_upkeep` | inteiro | Custo mensal da corte; padrão 20 |

Campos opcionais ausentes usam os padrões acima.

**Província**

| Campo | Tipo | Descrição |
| --- | --- | --- |
| `name` | string | Nome exibido |
| `owner` | string | Deve ser **exatamente** o `name` de uma facção |
| `map` | opcional | Posição no mapa hexagonal: `Some((col: i32, row: i32))` |
| `development` | inteiro | 50 = neutro; afeta receita |
| `unrest` | inteiro | Descontentamento inicial (0–100) |
| `peasant` | inteiro | População camponesa |
| `burgher` | inteiro | População burguesa |
| `noble` | inteiro | População nobre |
| `clergy` | inteiro | População religiosa |

## Como adicionar conteúdo

1. Adicione a facção (se necessário) em `factions`.
2. Adicione a província em `provinces`, referenciando o `owner` pelo nome.
3. Rode `cargo run -p crownveil-cli` e confira se o mundo carrega.
4. Rode `cargo test` para garantir que nada quebrou.

O loader rejeita `owner` que não corresponda a nenhuma facção, retornando
`LoadError::UnknownFaction` com o nome da província e da facção. Se `map` for
omitido, a província recebe uma posição automática em grade (4 colunas).

## Eventos e modificadores

`content/events.ron` define modificadores e eventos. É carregado com
`load_events(source, world)` depois de `load_world`.

```ron
(
    modifiers: [
        (key: "seca", name: "Seca", unrest: 3, food_pct: -20),
    ],
    events: [
        (
            key: "seca",
            title: "Seca",
            description: "Os rios minguam e a terra racha sob o sol.",
            scope: Province,
            chance: 3,
            cooldown_months: 60,
            conditions: [LacksModifier("colheita_farta")],
            effects: [AddModifier("seca", Some(12)), Unrest(3)],
        ),
    ],
)
```

**Modificador**: `key` (único), `name`, e efeitos opcionais (padrão 0):
`unrest`, `income_pct`, `food_pct` (só províncias), `legitimacy` (por mês, só
facções).

**Evento**:

| Campo | Descrição |
| --- | --- |
| `key` | Identificador único |
| `title`, `description` | Texto exibido na Crônica |
| `scope` | `Province` ou `Faction` |
| `chance` | Chance mensal por alvo, em milésimos (0–1000) |
| `cooldown_months` | Meses até repetir no mesmo alvo; padrão 120 |
| `conditions` | Todas precisam valer (opcional) |
| `effects` | Aplicados quando o evento ocorre (opcional se houver `options`) |
| `options` | Escolhas: `(label: "...", effects: [...])`; zero ou pelo menos duas |

Com `options`, a facção que decide escolhe uma. **A primeira opção é a escolha
da IA** e a padrão quando o jogador não responde em 12 meses; ordene as opções
pensando nisso.

```ron
(
    key: "profeta_errante",
    title: "Profeta Errante",
    scope: Province,
    chance: 4,
    conditions: [ClergyAtLeast(300)],
    effects: [Unrest(3)],
    options: [
        (label: "Acolher o profeta", effects: [AddModifier("fervor_religioso", Some(24))]),
        (label: "Expulsá-lo da província", effects: [Unrest(4), Legitimacy(1)]),
    ],
)
```

Condições: `UnrestAtLeast(n)`, `UnrestBelow(n)`, `DevelopmentAtLeast(n)`,
`DevelopmentBelow(n)`, `ClergyAtLeast(n)`, `Famine`, `NoFamine` (só
província); `LegitimacyAtLeast(n)`, `LegitimacyBelow(n)`, `TreasuryAtLeast(n)`,
`TreasuryBelow(n)`, `TaxAtLeast(n)`, `FaithAtLeast(n)`, `FaithBelow(n)`,
`ManaAtLeast(n)`, `ManaBelow(n)`, `AtWar`, `AtPeace`, `HasModifier("k")`,
`LacksModifier("k")`.

Efeitos: `Unrest(n)`, `Development(n)`, `Population(pct)`, `Legitimacy(n)`,
`Treasury(n)`, `Army(n)`, `Faith(n)`, `Mana(n)`, `AddModifier("k", Some(meses))`
(ou `None` para permanente), `RemoveModifier("k")`,
`TriggerEvent("evento", meses)`.

### Crises encadeadas

`TriggerEvent("chave", meses)` agenda outro evento no mesmo alvo. A
continuação ignora a `chance`, mas só acontece se as `conditions` dela ainda
valerem — é assim que o jogador desarma uma crise. Use `chance: 0` para eventos
que só existem como continuação. A chave pode apontar para um evento definido
mais abaixo no arquivo.

```ron
(
    key: "martires_da_revolta",
    title: "Os Mártires da Revolta",
    scope: Province,
    chance: 0,
    conditions: [UnrestAtLeast(40)],
    options: [
        (label: "Proibir as homenagens", effects: [TriggerEvent("insurreicao", 12)]),
        (label: "Erguer um memorial", effects: [Treasury(-150), Unrest(-10)]),
    ],
)
```

Como a IA sempre escolhe a primeira opção, a ordem decide se ela escala a
crise ou não. Na UI, o tooltip da opção mostra só "consequências em N meses".
Um efeito de evento ou rito **de facção** não pode encadear evento de província.

O loader rejeita chaves duplicadas, modificadores desconhecidos, condições de
província em eventos de facção, `chance` fora de 0–1000, cooldown negativo,
duração não positiva, `Population` abaixo de −100, eventos sem efeitos nem
opções, eventos com uma única opção, opções sem `label`, `TriggerEvent` para
evento desconhecido, com menos de 1 mês ou de facção para província.

## Ritos e decretos

`content/rites.ron` define as ações pagas do jogador. Decretos são ritos pagos
só com ouro (Obras Públicas, Festival Real, Doação ao Clero, Subsídio aos
Magos): dão destino ao tesouro, que perde 2% ao mês para a corrupção. É carregado com
`load_rites(source, world)` **depois** de `load_events`, porque ritos
referenciam modificadores pela chave.

```ron
(
    rites: [
        (
            key: "selar_o_veu",
            name: "Selar o Véu",
            description: "Um círculo de magos costura a fenda com fios de luz.",
            scope: Province,
            mana_cost: 120,
            cooldown_months: 12,
            conditions: [HasModifier("veu_rasgado")],
            effects: [RemoveModifier("veu_rasgado"), Unrest(-5)],
        ),
    ],
)
```

| Campo | Descrição |
| --- | --- |
| `key`, `name`, `description` | Identificador único e textos exibidos |
| `scope` | `Province` (província da própria facção) ou `Faction` |
| `faith_cost`, `mana_cost`, `gold_cost` | Custos; padrão 0, nunca negativos |
| `cooldown_months` | Meses até repetir no mesmo alvo; padrão 0 |
| `conditions` | Mesmas condições dos eventos (opcional) |
| `effects` | Mesmos efeitos dos eventos; pelo menos um |

Modificadores também aceitam `faith` e `mana` (por mês).

## Saves

Saves são RON gerados por `content::save::save_world` e lidos por `load_save`:

```ron
(
    format_version: 1,
    game_version: "0.1.0",
    world: ( ... ),   // o World inteiro, incluindo definições de eventos e ritos
)
```

- O `World` vai inteiro: estado, fila de comandos, decisões pendentes, crises
  agendadas, RNG e **as definições** de modificadores, eventos e ritos. Mudar
  `content/*.ron` depois não altera partidas salvas; o conteúdo novo vale para
  jogos novos.
- `load_save` lê primeiro só `format_version`: versão atual carrega; versão
  maior é recusada (`NewerVersion`); versões antigas precisarão de migração.
- `write_save_file` grava em `<arquivo>.tmp` e renomeia, para nunca deixar um
  save pela metade no lugar do anterior.
- Carregar e continuar produz exatamente o mesmo resultado que não ter salvo
  (há teste para isso).

### Migrações

Saves de versão antiga passam por uma cadeia de migrações antes da leitura
tipada: `save::doc` lê o RON numa árvore que **preserva nomes de variantes**
(`ron::Value` perde `Province(3)` → `[3]`), cada `Migration { from: n }` leva
da versão *n* para *n+1* editando essa árvore, e o resultado é lido com os
tipos atuais.

Quando mudar o formato de forma não aditiva:

1. Suba `SAVE_FORMAT_VERSION` em `crates/content/src/save.rs`.
2. Adicione a migração em `crates/content/src/save/migrations.rs`, usando os
   auxiliares de `Node` (`rename_field`, `set_field`, `remove_field`,
   `for_each_in_world_list`…).
3. Gere um save da versão nova em `crates/content/tests/fixtures/save_v<N>.ron`
   (por exemplo, com `save` no REPL) e **não altere os antigos**.

O teste `every_released_save_version_still_loads` carrega todas as fixtures de
1 até a versão atual e avança 2 anos; se uma mudança quebrar saves antigos sem
migração, ele falha. Campos novos com `#[serde(default)]` não exigem nada disso.

O REPL salva em `crownveil.save.ron` (ou no arquivo informado); a UI usa
`$XDG_DATA_HOME/crownveil/saves/quicksave.save.ron` (padrão
`~/.local/share/...`).

### Diplomacia

Hoje as relações entre facções começam em 0 e não há guerras iniciais; guerras
só surgem por comandos (`DeclareWar`). Quando o formato for exposto, relações e
tréguas iniciais entrarão como um bloco opcional `diplomacy`.

## Evolução planejada

- **Culturas e raças** (`content/cultures/`): tags que afetam crescimento,
  produção e descontentamento.
- **Localização** (`content/loc/`): textos separados dos identificadores.
- **Validação** (`cargo test`): checar referências, faixas e nomes duplicados.

Regra de ouro: adicionar conteúdo **não deve** exigir mudar código de regra.
Se exigir, provavelmente falta um campo/tag genérico no modelo de dados.
