# Glossário

| Termo | Significado |
| --- | --- |
| **Tick** | Unidade atômica de simulação. Hoje, 1 tick = 1 mês. |
| **Comando** | Intenção do jogador (ex.: subir imposto) aplicada pela simulação. |
| **Facção** | Entidade política jogável ou controlada pela IA (reino, corte, domínio). |
| **Província** | Unidade territorial com população, desenvolvimento e dono. |
| **Pop / População** | Grupo populacional de uma província, dividido em classes. |
| **Classe populacional** | `peasant` (camponês), `burgher` (burguês), `noble` (nobre), `clergy` (clero). |
| **Desenvolvimento** | Potencial econômico da província. 50 = neutro. |
| **Descontentamento (unrest)** | Insatisfação local (0–100); alta reduz crescimento. |
| **Legitimidade** | Aceitação do governo pela população (0–100); afeta o descontentamento. |
| **Tesouro** | Recurso monetário da facção, em unidades inteiras. |
| **Fé** | Recurso da facção (0–1000) vindo do clero; acalma as províncias e paga ritos. |
| **Magia** | Recurso da facção (0–1000) vindo do desenvolvimento; paga ritos, mas acumulada assusta o povo. |
| **Rito** | Ação do jogador paga com fé, magia e/ou ouro, definida em `content/rites.ron`. |
| **Evento** | Acontecimento orientado a dados (condições + chance + efeitos), às vezes com escolhas. |
| **Modificador** | Bônus/malus temporário aplicado a uma entidade (magia, fé, clima etc.). |
| **Seed** | Semente do RNG; garante reprodutibilidade da simulação. |
| **Determinismo** | Mesma seed + mesmas entradas ⇒ mesmo resultado, sempre. |
| **Headless** | Execução da simulação sem interface gráfica. |
| **Composition root** | Crate que monta o mundo e orquestra (`cli`, `client`). |
| **RON** | Formato de dados legível usado para conteúdo do jogo. |
