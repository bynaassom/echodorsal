# Design — echodorsal

Identidade visual **dark-first** para o painel web, o menu bar e (v2) a Touch Bar.
O brief do projeto vence qualquer default: paleta, fontes e ícones abaixo são decisões, não sugestões.

## 1. Referências e o que aproveitamos

> **Limite da análise:** foi feita a partir do texto e dos metadados das páginas, sem renderização.
> Tipografia, espaçamento e imagens **não foram vistos**. Validar cada ponto com prints antes da Fase 2.
> A referência GitHub foi citada sem link e não foi analisada aqui.

| Referência | O que observei | O que aproveitar |
|---|---|---|
| **Consensys** | cor de tema neon-lime (`#c1ff14`) sobre base escura; tom institucional e de autoridade; sequência de 3 etapas ("do design à operação"); blocos editoriais com afirmações fortes | lime funciona em contexto sério se usado com contenção; numeração **só** quando o conteúdo é uma sequência real (ciclo de vida de uma task) |
| **Cloudflare** | cor de tema laranja (`#ff4801`); números grandes com rótulo curto (latência, volume); cartões de planos lado a lado; visual "wall clock vs CPU time" contrastando espera e trabalho; mock de terminal com agentes em segundo plano e contagem de tokens por agente | feed de agentes com uso por agente; linha do tempo "esperando vs trabalhando"; cartões de limites lado a lado; números como conteúdo |
| **Dropshot** | cor de tema quase preta (`#0a0a0e`); proposta de um único propósito ("cinematic product shots", roda no navegador); preço único | telas com foco único e muito respiro; profundidade em quase-preto; movimento contido. *Corpo da página não carregou na análise: confirmar com prints* |

## 2. Plano de design

**Cor** — base de 5 valores nomeados + derivados (seção 3):
Ink `#161925`, Navy `#23395b`, Steel `#406e8e`, Mist `#8ea8c3`, Lime `#dcf763`.

**Tipo** — Work Sans (interface e títulos) + JetBrains Mono (dados: IDs, números, tempos, comandos).

**Layout** — tela única de comando: a primeira coisa na Visão geral é a **resposta** à pergunta "como está meu projeto?",
não uma parede de widgets. Alinhamento à esquerda, colunas assimétricas, densidade de console com respiro de produto.

**Princípios**
1. **Resposta primeiro.** O estado atual numa frase legível; detalhes abaixo.
2. **Uma coisa memorável: o ping de sonar.** Um anel lime que se expande uma vez quando chega um comando de voz ou um worker começa. Todo o resto é quieto.
3. **Estrutura é informação.** Bordas, divisórias e numeração só quando codificam algo.
4. **Dados em mono, texto em Work Sans.** Mono para valores, nunca como decoração de rótulo.
5. **Lime é escasso.** Ação primária, foco, "em curso". Nunca alerta.

**Autocrítica contra defaults.** Fundo quase preto + acento verde-ácido é um padrão comum; a escolha vem do brief, então a diferenciação está em:
bases tingidas de azul (não preto neutro), lime ocupando poucos pixels, o ping de sonar como assinatura, raios de borda hierárquicos,
cartões que não são todos iguais, e ausência dos vícios listados na seção 9.

## 3. Tokens

```css
:root {
  /* superfícies */
  --bg-0: #161925;      /* Ink — fundo */
  --bg-1: #1a2335;      /* superfície elevada (30% Navy sobre Ink) */
  --bg-2: #23395b;      /* Navy — seleção, hover, destaque */
  --border: #293f54;    /* divisórias sutis (decorativo) */
  --steel: #406e8e;     /* bordas de input, foco, séries de gráfico */

  /* texto */
  --text-1: #f1f5f8;    /* principal */
  --text-2: #8ea8c3;    /* Mist — secundário */
  --text-3: #7690ab;    /* auxiliar; só em bg-0/bg-1 */

  /* acento e semântica */
  --accent: #dcf763;    /* Lime — ação primária, foco, em curso */
  --on-accent: #161925; /* texto sobre lime */
  --ok: #4fd1a5;        /* concluída */
  --warn: #f5a524;      /* bloqueada, atenção */
  --danger: #ff7a7a;    /* falha */

  /* tipo */
  --font-ui: "Work Sans", system-ui, sans-serif;
  --font-mono: "JetBrains Mono", ui-monospace, monospace;

  /* raios (hierarquia, não um raio único) */
  --r-control: 6px; --r-card: 10px; --r-pop: 14px; --r-chip: 999px;
}
```

### Contraste (WCAG, calculado)

| Par | Razão | Uso |
|---|---|---|
| `accent` sobre `bg-0` / `bg-2` | 14,6:1 / 9,7:1 | ok para texto |
| `on-accent` sobre `accent` | 14,6:1 | botão primário |
| `text-1` sobre `bg-0` / `bg-2` | 15,9:1 / 10,6:1 | ok |
| `text-2` sobre `bg-0` / `bg-2` | 7,1:1 / 4,7:1 | ok (no limite sobre `bg-2`) |
| `text-3` sobre `bg-0` | 5,3:1 | só em `bg-0`/`bg-1` |
| `steel` sobre `bg-0` | 3,2:1 | **só gráfico/borda**, nunca texto |
| `steel` sobre `bg-1` | 2,9:1 | abaixo de 3:1: campos ficam sobre `bg-0` ou ganham borda mais clara |
| `ok` / `warn` / `danger` sobre `bg-0` | 9,2 / 8,6 / 6,9 | ok |
| `ok` / `warn` / `danger` sobre `bg-2` | 6,1 / 5,7 / 4,6 | ok |

Chips de status: cor semântica como texto sobre ela mesma a ~15% de opacidade (todos acima de 4,5:1).
Simulação de daltonismo **não foi feita**: por isso status sempre com ícone e rótulo.

### Mapeamento de status

| Status (Orca) | Rótulo na UI/voz | Cor | Tratamento |
|---|---|---|---|
| `pending`, `ready` | Próxima | `text-2` / `steel` | contorno |
| `dispatched` | Em curso | `accent` | ponto com ping |
| `completed` | Concluída | `ok` | ícone de check |
| `failed` | Falhou | `danger` | ícone de alerta |
| `blocked` | Bloqueada | `warn` | ícone de pausa |
| gate pendente | Aguardando você | `accent` + borda | destaque no topo |

## 4. Tipografia

Self-host em WOFF2 (subset latin + latin-ext), embutidas no binário; `font-display: swap`. Confirmar licenças (ambas costumam ser OFL) e
a disponibilidade de números tabulares na Work Sans; JetBrains Mono já alinha colunas.

| Papel | Fonte | Tamanho/linha | Peso |
|---|---|---|---|
| Título de tela | Work Sans | 28/36 | 600 |
| Título de seção | Work Sans | 20/28 | 600 |
| Subtítulo/cartão | Work Sans | 16/24 | 600 |
| Corpo | Work Sans | 14/20 | 400 |
| Auxiliar | Work Sans | 12/16 | 400–500 |
| Dados (IDs, %, tempos, comandos) | JetBrains Mono | 13/20 | 400–500 |

Linhas de texto < 80 caracteres. Sentence case; sem caixa alta para rótulos.

## 5. Ícones e marca

- **Biblioteca:** Dinkie Icons (atelierAnchor, MIT, monocromática, ~1.200 ícones, via Iconify: `dinkie-icons:<nome>`).
  Usar a variante **padrão** (sem `-small`, sem `-filled`) em todo o app.
- **Marca:** `dinkie-icons:spouting-whale` (baleia). **Ainda não verificado** que o slug resolve: a tarefa F0-08 deve falhar o build se não existir.
- **Pipeline:** extração em *build time* para um sprite SVG com `currentColor`, embutido no binário; partial Askama `icon("nome")`.
  **Sem** web component do Iconify em runtime (busca na rede; quebra o local-first e a CSP).
- Mapeamento nome-lógico → slug em `web/icons.map.json`; qualquer slug ausente quebra o build.
- Tamanhos 16/20/24 px. Testar legibilidade da baleia em 16 px; se falhar, simplificar o uso (sem detalhes finos) em vez de trocar de família.
- **Licença:** manter aviso MIT e crédito a atelierAnchor em `THIRD_PARTY_LICENSES.md` e na tela "Sobre".

### Ícone do app (macOS)
Master 1024 px: squircle com gradiente Ink → Navy, baleia em Lime (~58% do lado, margens de segurança). Gerar `.icns`.

### Menu bar
*Template image* monocromática da baleia (~18 pt, @1x/@2x; confirmar diretrizes). Estado por **ponto** colorido (ok/warn/danger/accent);
o Lime não aparece em template, por isso o ponto.

### Touch Bar (v2)
Um ícone no Control Strip: baleia + anel de uso, expandindo ao toque. Fundo preto da Touch Bar favorece o Lime. Testar em 16/20/24 px.

## 6. Telas (wireframes de intenção)

### Visão geral

```
┌─────────────────────────────────────────────────────────────────┐
│ [baleia] echodorsal            Projeto: [ checkout v ]   Orca online │
├─────────────────────────────────────────────────────────────────┤
│ Agora                                                             │
│ 2 em curso, 1 aguardando você, nenhuma falha                      │
│                                                                   │
│ ┌ Aguardando você ─────────────┐  ┌ Em curso ───────────────────┐ │
│ │ Mesclar mudança do botão?    │  │ (ping) Auditoria de billing │ │
│ │ [ Sim ]  [ Não ]             │  │ codex, há 14 min            │ │
│ └──────────────────────────────┘  │ Ajustar footer mobile       │ │
│                                   │ agy, há 3 min               │ │
│ Limites                           └─────────────────────────────┘ │
│ Codex, janela de 5 h  [#######---] 72%   reseta em 1 h 40         │
│ agy, cota do grupo    [###-------] 31%                            │
└─────────────────────────────────────────────────────────────────┘
```

### Tarefas
Lista com filtros (Próximas, Em curso, Concluídas, Falhas, Bloqueadas) e, por tarefa, uma barra de tempo que separa **espera** e **trabalho**
(inspirada no "wall clock vs CPU"). O ciclo de vida de uma task é uma sequência real: aqui a numeração de etapas é legítima.

### Limites
Cartões lado a lado por provedor (inspirados nos planos da Cloudflare), cada um com suas janelas (5 h, semanal, requisições/dia),
histórico em gráfico pequeno e editor de políticas (alerta a 80%, roteamento, bloqueio).

### Sistema
Versão do Orca e compatibilidade, estado do túnel, últimas chamadas do Alexa (intent, resultado, latência), fila de eventos.

## 7. Componentes

`StatusChip`, `UsageBar`, `TaskRow`, `GateCard`, `AgentFeedRow` (agente, tarefa, uso), `EmptyState`, `ErrorNotice`, `CommandPalette` (⌘K).
Cada um com variantes documentadas e testes visuais simples. Cartões **não** são todos iguais: Gate tem destaque; Em curso tem densidade; Limites tem gráfico.

## 8. Movimento

- **Assinatura:** o ping de sonar (anel Lime expandindo ~600 ms, uma vez) em comando de voz recebido e worker iniciado.
- Valores ao vivo interpolam em ~200 ms. Interação do usuário (abrir, expandir, confirmar) anima para mostrar o que mudou.
- **Sem** entrada animada por seção, **sem** hover animado em todo cartão.
- `prefers-reduced-motion`: ping vira realce estático.

## 9. Evitar (vícios de página genérica)

- Rótulo em caixa alta espaçada acima de cada título; textos meta unidos por pontos medianos; "→" colado em botões e links.
- Numeração (01/02/03) quando o conteúdo não é sequência.
- Mesmo raio, mesma sombra cinza e mesmos cartões em tudo; gradientes decorativos.
- Mono como enfeite de rótulo pequeno. Mono é para **dados**.
- Destacar uma única palavra do título com cor/itálico.

## 10. Copy (pt-BR)

- Sentence case, verbos ativos, vocabulário idêntico entre painel e voz: **Próximas, Em curso, Concluídas, Falhou, Bloqueada, Aguardando você**.
- Botões dizem o que acontece: "Aprovar mesclagem", não "Enviar". A confirmação repete o verbo: "Mesclagem aprovada".
- Erros dizem o que houve e como resolver, sem pedir desculpa e sem vagueza: "O Orca não está aberto. Abra o app e tente de novo."
- Estados vazios convidam a agir: "Nenhuma tarefa em curso. Crie uma tarefa no Orca ou por voz (Fase 4)."
- Sem emojis na interface.

## 11. Acessibilidade e qualidade mínima

Foco visível (anel `accent`), alvos ≥ 32 px, contraste conforme a tabela, status nunca só por cor, navegação por teclado completa,
responsivo até mobile (o painel pode ser aberto em outro dispositivo da rede local no futuro), `prefers-reduced-motion` respeitado.
