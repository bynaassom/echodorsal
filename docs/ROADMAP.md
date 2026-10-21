# Roadmap — echodorsal

Uma fase por vez. Cada fase só termina quando o **critério de pronto** é demonstrável. O que não está aqui vai
para o backlog (final do arquivo), não para o código.

## Visão geral

| Fase | Nome | Entrega principal | Pronto quando |
|---|---|---|---|
| 0 | Spikes | Pipeline voz → servidor → Orca provado | Alexa diz "Orca online, N worktrees" |
| 1 | MVP somente leitura | Status, em curso, concluídas, próximas, gates pendentes | Você pergunta e entende a resposta sem olhar a tela |
| 2 | Painel local | Histórico, linha do tempo, saúde do sistema | "Tarefas anteriores" vem do seu log, não do estado do Orca |
| 3 | Limites e menu bar | Leitura de uso, políticas, relatório falado, menu bar | Você ouve "Codex em 72%, reseta em 1h40" e vê o mesmo no menu bar |
| 4 | Escrita segura | Criar task, iniciar worker, responder gates por voz | Nenhuma ação irreversível sem confirmação |
| 5 | Polimento | Docs, demo, hardening, release | Alguém clona e roda em 15 minutos |

## Fase 0 — Spikes (de-risk)

Objetivo: provar as partes incertas antes de construir em cima delas.

Estado: F0-01 (workspace e CI) preparado; verificações locais de Rust, `cargo deny` e build do front passaram. O CI do GitHub ainda precisa confirmar a branch.

- Skill "olá" em pt-BR respondendo via túnel, com validação de assinatura.
- `OrcaAdapter` lendo `status`, `worktree ps` e `task-list`, com fixtures gravadas.
- Investigação da **fonte de uso** (Orca, `agy`, Codex) e do **`agy` headless** (latência, rastros locais).
- Pipeline de ícones (`dinkie-icons:spouting-whale`) e tokens de design.

Riscos: reconhecimento de fala em pt-BR; CLI do Orca instável (orquestração é experimental); fonte de uso inexistente.
Especificações: `docs/tasks/fase-0.md`.

## Fase 1 — MVP somente leitura

- Intents: status por projeto, em curso, concluídas (hoje/ontem/semana), próximas, gates pendentes.
- Cobertura dos **dois modelos** do Orca: tasks de orquestração e worktrees comuns (status do card + comentário).
- Event log em SQLite (alimentado por snapshots e por `orchestration check --wait`).
- Respostas degradadas úteis: Orca fechado, Mac sem rede, comando que falha.

Pronto: 10 perguntas reais ao longo de um dia retornam respostas corretas; casos de erro falam o que fazer.

## Fase 2 — Painel local

- Painel SSR (axum + Askama) com HTMX e SSE; TypeScript só para ilhas (gráficos, paleta de comandos).
- Telas: Visão geral, Tarefas (linha do tempo e filtros), Sistema (saúde, versão do Orca, tunnel, últimas chamadas do Alexa).
- Aplicação dos tokens e ícones de `docs/DESIGN.md`. Painel apenas em `127.0.0.1`.

Pronto: histórico de tarefas sobrevive a `orchestration reset`; painel atualiza sem recarregar.

## Fase 3 — Limites e menu bar

- Fonte de uso definida na Fase 0 (ADR), com janelas modeladas por provedor (5 h, semanal, requisições/dia).
- Políticas: alerta ao passar de 80%, roteamento de agente por uso, bloqueio opcional de novos dispatches.
- Relatório falado de uso. Tela Limites no painel.
- App Swift de menu bar: ícone de baleia (template) + ponto de estado; popover com uso e tarefas em curso.
  Assina o stream do servidor; sem polling.

Pronto: o mesmo número aparece na voz, no painel e no menu bar; CPU do app Swift ≈ 0 em repouso.

## Fase 4 — Escrita segura

- Criar task por voz: ditado → releitura → confirmação → `task-create`.
- Iniciar worker, responder gates (sim/não) por voz, PIN falado para ações sensíveis.
- Modo manual de permissões para workers iniciados por voz; log de auditoria de toda escrita.
- Flag `writes` desligada por padrão.

Pronto: tentativa de ação sem confirmação é recusada e auditada; teste ponta a ponta com binário `orca` falso cobre todos os caminhos de escrita.

## Fase 5 — Polimento

- README com demo (GIF/vídeo), diagrama de arquitetura, ADRs revisados.
- Matriz de compatibilidade com versões do Orca; guia de instalação (túnel, skill, Keychain).
- Hardening: rate limit em `/alexa`, revisão de logs sem segredo, `cargo audit`/`deny` no CI.
- Release com binário e app Swift assinado para uso pessoal.

## Backlog (fora do v1)

- **Touch Bar (v2):** widget no Control Strip com um ícone (anel de uso + baleia) que expande ao toque.
  Usa API privada, só roda a partir de um `.app` e não serve para a App Store; isolado em pacote próprio
  com feature flag e risco documentado. Verificar suporte da versão do macOS em uso.
- **Refiner (v1.x):** reparo do ditado e releitura (`gemini-3.8-flash` variante `low` via `agy`), resumo do dia,
  fallback de intent. Desligado por padrão; ver `docs/ARCHITECTURE.md`.
- Tema claro (remapeamento dos tokens).
- Notificações proativas por voz (sujeito a suporte do Alexa em pt-BR; não verificado).

## Fora de escopo

Modo conversa, provedores de LLM via API, app mobile, controle de alarme/música nativos, publicação na loja da Amazon.
