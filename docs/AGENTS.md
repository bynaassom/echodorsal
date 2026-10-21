# AGENTS.md — echodorsal

Instruções para agentes de código (Codex, Antigravity, Claude Code etc.) que trabalham neste repositório.
Leia este arquivo inteiro antes de qualquer tarefa. Em caso de conflito, **este arquivo e os ADRs vencem** o resto.

## O que é o projeto

**echodorsal** é um assistente de gerenciamento de projetos por voz.
O usuário fala com um Echo Pop (Alexa) e recebe respostas sobre o andamento de projetos que rodam no
[Orca](https://github.com/stablyai/orca) (IDE de agentes em worktrees paralelos), e, em fases avançadas,
dispara tarefas e responde a decision gates por voz.

Fluxo resumido:

```
Echo Pop → Alexa Custom Skill (pt-BR) → túnel HTTPS → servidor Rust local → CLI do Orca
                                                        ├─ painel web (HTMX + TS)
                                                        ├─ event log (SQLite)
                                                        └─ menu bar (Swift) / Touch Bar (v2)
```

**Não é um chatbot.** Não há modo conversa livre. Voz serve para consultas curtas e comandos confirmados.

## Regras invioláveis

1. **Somente leitura por padrão.** Qualquer operação de escrita no Orca fica atrás de feature flag
   (`writes`) e exige confirmação (decision gate ou PIN falado). Ver `docs/ORCA-INTEGRATION.md`.
2. **Nunca invente comandos ou flags do Orca.** Descubra com `orca agent-context --json` e
   `orca skills get orca-cli`. Se um comando não existir na versão instalada, falhe com erro claro.
3. **O túnel expõe apenas `/alexa`.** Painel e API ficam em `127.0.0.1`. Nunca mude isso sem ADR.
4. **Toda requisição em `/alexa` é validada** (assinatura, cadeia de certificado, timestamp, `applicationId`,
   `userId` na allowlist) antes de qualquer lógica. Sem exceções, nem em dev.
5. **Sem LLM no caminho síncrono da voz.** O limite do Alexa é ~8 s por turno. LLM só em fluxos assíncronos
   e opcionais (`Refiner`), e desligado por padrão.
6. **Sem chaves de API de LLM.** O projeto usa assinaturas via CLIs oficiais (Codex, `agy`). Segredos
   (tokens, PIN) ficam no Keychain do macOS, nunca no repositório, em logs ou em fixtures.
7. **Respostas de voz curtas**: no máximo 3 frases. Nunca ler caminhos, hashes, código ou stack traces.
8. **Sem `unwrap()`/`expect()` fora de testes.** Erros tipados (`thiserror`) nas bibliotecas, `anyhow` só nos binários.

## O que NÃO fazer (escopo)

- Modo conversa, chat, memória de conversa, ou provedores de LLM via API.
- Reimplementar rastreio de uso que o Orca já faz, ou o app mobile do Orca.
- Controlar alarme/música nativos da Alexa (a skill não consegue).
- Publicar a skill na loja da Amazon (fica em modo de desenvolvimento).
- Tocar na Touch Bar antes da v2 (usa API privada; ver `docs/ROADMAP.md`).
- Adicionar dependências pesadas sem justificativa em ADR.

## Estrutura do repositório

```
echodorsal/
├─ AGENTS.md
├─ Cargo.toml                  # workspace
├─ crates/
│  ├─ echodorsal-domain/       # tipos puros, sem I/O
│  ├─ echodorsal-orca/         # adaptador da CLI do Orca + parsers
│  ├─ echodorsal-alexa/        # modelos de request/response, validação, SSML
│  ├─ echodorsal-store/        # SQLite: event log, snapshots, auditoria
│  ├─ echodorsal-usage/        # fontes de uso e políticas de limite
│  ├─ echodorsal-refiner/      # trait Refiner (+ Noop; agy na v1.x)
│  └─ echodorsal-server/       # axum: /alexa, /api, SSE, painel SSR
├─ web/                        # TypeScript (ilhas) + CSS, empacotado com Vite
├─ apps/macos/                 # Swift: menu bar (v1), Touch Bar (v2)
├─ alexa/                      # modelo de interação pt-BR, skill.json
├─ fixtures/orca/              # saídas JSON gravadas do Orca (sem dados sensíveis)
├─ scripts/
├─ docs/
└─ THIRD_PARTY_LICENSES.md
```

## Comandos

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check            # licenças e advisories
pnpm --dir web install && pnpm --dir web build   # front (ver ADR 0001)
```

Tudo isso precisa passar antes de abrir PR. Se uma ferramenta não estiver configurada, configure-a na tarefa F0-01.

## Definição de pronto

Uma tarefa só está pronta quando:
- os critérios de aceite da especificação estão atendidos **e demonstráveis** (por voz, painel ou teste);
- comandos acima passam; testes novos cobrem o comportamento novo;
- `docs/` e, se houver decisão cara de reverter, um ADR foram atualizados;
- nada de segredo, caminho pessoal ou dado de projeto real entrou no diff.

## Como trabalhar

- Uma tarefa por worktree, branch `feat/<id>-<slug>` (ex.: `feat/f0-05-orca-adapter`).
- Commits convencionais (`feat:`, `fix:`, `docs:`, `test:`, `refactor:`, `chore:`).
- PRs pequenos, descrevendo o *porquê*. Mudança de contrato com o Orca exige atualizar fixtures e a matriz de compatibilidade.
- Dúvida de escopo ou de produto: **pare e pergunte** (decision gate / `orca orchestration ask`) em vez de supor.
- Spikes produzem um relatório em `docs/spikes/` com decisão e evidências, não só código.

## Mapa de documentos

| Arquivo | Para quê |
|---|---|
| `docs/ROADMAP.md` | fases, entregas e critérios de pronto |
| `docs/ARCHITECTURE.md` | crates, portas, fluxos, modelo de dados, segurança |
| `docs/GUIDELINES.md` | padrões de código, testes, segurança, commits, ADRs |
| `docs/ORCA-INTEGRATION.md` | contrato com a CLI do Orca, fixtures, compatibilidade |
| `docs/VOICE-UX.md` | intents, frases em pt-BR, respostas e erros falados |
| `docs/DESIGN.md` | identidade visual, tokens, ícones, telas, copy |
| `docs/AGENT-WORKFLOW.md` | como orquestrar agentes neste projeto |
| `docs/tasks/fase-0.md` | especificações prontas das tarefas da Fase 0 |
| `docs/adr/` | decisões de arquitetura |
