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

### Diplomacia

Hoje as relações entre facções começam em 0 e não há guerras iniciais; guerras
só surgem por comandos (`DeclareWar`). Quando o formato for exposto, relações e
tréguas iniciais entrarão como um bloco opcional `diplomacy`.

## Evolução planejada

- **Eventos** (`content/events/`): condição + peso + efeitos.
- **Modificadores** (`content/modifiers/`): bônus temporários por tag.
- **Culturas e raças** (`content/cultures/`): tags que afetam crescimento,
  produção e descontentamento.
- **Localização** (`content/loc/`): textos separados dos identificadores.
- **Validação** (`cargo test`): checar referências, faixas e nomes duplicados.

Regra de ouro: adicionar conteúdo **não deve** exigir mudar código de regra.
Se exigir, provavelmente falta um campo/tag genérico no modelo de dados.
