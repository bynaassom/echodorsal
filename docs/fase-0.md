# Fase 0 — Especificações de tarefas

Cada seção é uma task independente para despachar a um worker (ver `docs/AGENT-WORKFLOW.md`).
Todos leem `AGENTS.md` primeiro. Spikes entregam relatório em `docs/spikes/`.

**Pronto da fase:** pergunta por voz "Orca online?" retorna "Orca online, N worktrees" (F0-09).

---

## F0-01 — Bootstrap do workspace e CI

**Contexto:** base para todas as outras tarefas. Ler `AGENTS.md`, `docs/GUIDELINES.md`, `docs/ARCHITECTURE.md`.
**Escopo:**
1. Criar o workspace Cargo com os crates vazios descritos em `AGENTS.md` (apenas `lib.rs`/`main.rs` mínimos).
2. `[workspace.lints]` (clippy estrito), `rust-version`, `rustfmt.toml`.
3. CI (GitHub Actions): `fmt --check`, `clippy -D warnings`, `test`, `cargo deny check`.
4. `deny.toml` com licenças permitidas; `THIRD_PARTY_LICENSES.md` inicial; `echodorsal.example.toml`.
5. `web/` com Vite + TypeScript vazios e script `pnpm build` funcionando.
**Fora do escopo:** qualquer lógica de negócio.
**Critérios de aceite:** CI verde em branch limpa; `cargo test --workspace` roda (mesmo que vazio); `pnpm --dir web build` gera saída.
**Verificação:** abrir PR e ver o CI verde.
**Entregáveis:** workspace, CI, `deny.toml`, `THIRD_PARTY_LICENSES.md`, `web/`.
**Depende de:** —

---

## F0-02 — Spike: skill "olá" em pt-BR

**Contexto:** provar o pipeline de voz e o reconhecimento em pt-BR. Ler `docs/VOICE-UX.md`.
**Escopo:**
1. Modelo de interação mínimo em `alexa/` (locale pt-BR) com nome de invocação "eco dorsal", intents `SaudeSistemaIntent`, `AMAZON.HelpIntent`, `AMAZON.StopIntent`, `AMAZON.FallbackIntent`.
2. Handler `POST /alexa` em `echodorsal-server` devolvendo SSML ("Eco dorsal no ar.") e tratando `LaunchRequest`/`SessionEndedRequest`.
3. Validação **ainda desligada por flag explícita de dev** (será obrigatória na F0-03).
4. Relatório `docs/spikes/skill-ola.md`: pt-BR aceito? nome de invocação aceito? 20 frases testadas com taxa de acerto por intent; comportamento do slot de texto livre (frase-veículo).
**Fora do escopo:** Orca, painel, escrita.
**Critérios de aceite:** "Alexa, abrir eco dorsal" responde; relatório com números reais.
**Verificação:** demonstração no Echo Pop (modo de desenvolvimento) e leitura do relatório.
**Entregáveis:** `alexa/`, handler, `docs/spikes/skill-ola.md`, atualização de `docs/VOICE-UX.md` se algo divergir.
**Depende de:** F0-01

---

## F0-03 — Validação de requisições Alexa

**Contexto:** segurança obrigatória (`AGENTS.md`, regra 4). Seguir a documentação oficial vigente da Amazon sobre validação de requisições.
**Escopo:**
1. Validar URL e cadeia do certificado informados na requisição, validade do certificado e nome esperado, e a **assinatura do corpo bruto** (confirmar na doc qual cabeçalho/algoritmo está vigente).
2. Validar `timestamp` dentro da tolerância recomendada pela Amazon.
3. Conferir `applicationId` e `userId` contra a allowlist do `echodorsal.toml`.
4. Cache de certificados com expiração; falha segura (rejeitar em caso de dúvida).
5. Remover/neutralizar a flag de dev da F0-02 para uso fora de testes locais explícitos.
**Fora do escopo:** autenticação do painel.
**Critérios de aceite:** testes cobrindo requisição válida, assinatura inválida, certificado inválido, timestamp velho, `applicationId`/`userId` fora da allowlist; todos os inválidos retornam `401` sem corpo útil.
**Verificação:** `cargo test -p echodorsal-alexa`; teste manual com requisição adulterada.
**Entregáveis:** módulo de validação, testes, seção de segurança atualizada em `docs/ARCHITECTURE.md`.
**Depende de:** F0-02

---

## F0-04 — Túnel e exposição mínima

**Contexto:** expor apenas `/alexa`. Ler `docs/ARCHITECTURE.md` (segurança).
**Escopo:**
1. Documentar e scriptar um túnel (preferência: Cloudflare Tunnel nomeado) mapeando **somente** `/alexa` para o servidor local.
2. Servidor com `bind` em `127.0.0.1`; rotas de painel/API inacessíveis pelo túnel (teste automatizado).
3. Segredos do túnel no Keychain; nada no repositório.
4. Relatório `docs/spikes/tunel.md` (latência medida, comportamento com Mac em repouso, passos de recuperação).
**Fora do escopo:** domínio público para o painel.
**Critérios de aceite:** a skill responde via túnel; requisição externa a `/api` ou ao painel é recusada; relatório com latências.
**Verificação:** `curl` externo em `/alexa` (assinatura inválida → `401`) e em `/api` (→ `404`/recusado).
**Entregáveis:** script, documentação, testes, relatório.
**Depende de:** F0-02

---

## F0-05 — Spike: `OrcaAdapter` e fixtures

**Contexto:** `docs/ORCA-INTEGRATION.md`. A orquestração é experimental.
**Escopo:**
1. `OrcaPort` + `CliOrca` em `echodorsal-orca`, com leitura de `status`, `worktree ps` e `orchestration task-list`/`gate-list`.
2. Verificação de **identidade e versão** no startup via `orca status --json` e `orca agent-context --json`.
3. Leitor tolerante (`serde`, enums com `Unknown`); timeouts de 3 s; `OrcaError::{NotRunning, Unsupported, Timeout, BadOutput}`.
4. Script `scripts/capture-orca-fixtures.sh` + fixtures sanitizadas em `fixtures/orca/<versao>/`.
5. Binário `orca` falso (`tests/support/fake-orca/`) com modos `ok`, `not-running`, `slow`, `bad-json`, `unknown-fields`.
6. Preencher a matriz de compatibilidade em `docs/ORCA-INTEGRATION.md` com a versão usada e o que foi confirmado/desconfirmado.
**Fora do escopo:** qualquer escrita; event loop (`check --wait`) — fica para a Fase 1.
**Critérios de aceite:** testes de contrato passam com as fixtures; testes com o falso cobrem os 5 modos; docs ajustadas aos comandos reais.
**Verificação:** `cargo test -p echodorsal-orca`; rodar o adaptador contra o Orca real e comparar.
**Entregáveis:** crate, script, fixtures, falso, matriz de compatibilidade.
**Depende de:** F0-01

---

## F0-06 — Spike: fonte de uso e limites

**Contexto:** decidir de onde vêm os números de uso (assinaturas, não API). Ler `docs/ORCA-INTEGRATION.md` (seção Uso).
**Escopo:** investigar e comparar, com evidências:
1. O Orca expõe uso por CLI? (procurar em `agent-context`; não confundir com `orca usage` de outros projetos.)
2. `agy`: comando/modo para consultar cota (há indício de `agy -p=/quota` em documentação de terceiros; **verificar**), janelas e grupos de modelos.
3. Codex: forma programática de obter os limites mostrados em `/status` (janela de 5 h e semanal), se existir.
4. Estabilidade, custo em cota da própria consulta, formato e frequência segura de leitura.
**Fora do escopo:** implementar políticas (Fase 3); qualquer método não oficial que automatize sessão web ou reaproveite tokens de interface.
**Critérios de aceite:** relatório `docs/spikes/uso.md` com tabela de fontes (como obter, formato, custo, riscos) e **recomendação**; rascunho de ADR `docs/adr/0002-fonte-de-uso.md`.
**Verificação:** revisão humana do relatório; exemplos reais de saída (sanitizados).
**Entregáveis:** relatório + ADR rascunho.
**Depende de:** F0-01

---

## F0-07 — Spike: `agy` headless para o Refiner

**Contexto:** `Refiner` opcional (reparo de ditado, resumo do dia). Gemini 3.8 Flash via `agy`.
**Escopo:** medir e documentar:
1. Modelos disponíveis (`agy models`) e variantes `low`/`medium`/`high` do `gemini-3.8-flash` na sua conta.
2. Modo não interativo: comando, formato de saída, tratamento de erro, código de saída.
3. Latência (frio e quente) em 10 execuções; uso de cota por chamada.
4. **Rastros locais**: onde `agy` grava registros de conversa, o que contém e como limpar.
5. Comportamento quando a cota acaba ou o modelo muda.
**Fora do escopo:** implementar o `Refiner` completo.
**Critérios de aceite:** relatório `docs/spikes/agy.md` com números e recomendação (viável no caminho assíncrono? sim/não; qual variante); rascunho `docs/adr/0003-refiner-agy.md`.
**Verificação:** revisão humana; comandos reproduzíveis no relatório.
**Entregáveis:** relatório + ADR rascunho + stub do trait `Refiner` com `NoopRefiner`.
**Depende de:** F0-01

---

## F0-08 — Spike: ícones e tokens de design

**Contexto:** `docs/DESIGN.md`.
**Escopo:**
1. Pipeline em build time: extrair `dinkie-icons:spouting-whale` (variante padrão) do Iconify para um sprite SVG com `currentColor`. O build **falha** se o slug não existir.
2. `web/icons.map.json` (nome lógico → slug) e partial Askama `icon("nome")`.
3. `web/tokens.css` com os tokens de `docs/DESIGN.md`; fontes Work Sans e JetBrains Mono self-hosted (WOFF2) e embutidas.
4. Página estática de prova (`/_design`, somente dev) mostrando tokens, tipografia, contrastes e a baleia em 16/20/24 px, em fundo `bg-0` e `bg-2`.
5. Verificar licenças (Dinkie Icons MIT; Work Sans e JetBrains Mono) e preencher `THIRD_PARTY_LICENSES.md`.
6. Gerar protótipos: ícone do app (master 1024 px) e template monocromático para menu bar.
**Fora do escopo:** telas reais do painel.
**Critérios de aceite:** build gera o sprite; `/_design` renderiza sem requisições externas; relatório curto `docs/spikes/design.md` dizendo se a baleia é legível em 16 px (com prints).
**Verificação:** abrir `/_design` com a rede desligada.
**Entregáveis:** pipeline, tokens, página de prova, licenças, protótipos de ícone, relatório.
**Depende de:** F0-01

---

## F0-09 — Voz → Orca, ponta a ponta

**Contexto:** fecha a Fase 0. Reúne F0-02 a F0-05.
**Escopo:**
1. Intent `SaudeSistemaIntent` chama `OrcaPort::status` e `worktrees` (com cache curto) e responde "Orca online, N worktrees" (usar os modelos de `docs/VOICE-UX.md`).
2. Orca fechado → "O Orca não está aberto. Abra o app no Mac e pergunte de novo."
3. Teste ponta a ponta com o binário falso nos modos `ok`, `not-running` e `slow`.
4. Medir latência p95 do endpoint em 30 chamadas.
**Fora do escopo:** demais intents.
**Critérios de aceite:** demonstração real no Echo Pop; testes verdes; p95 abaixo de 3 s registrado em `docs/spikes/e2e.md`.
**Verificação:** vídeo curto ou GIF da demonstração para o README futuro.
**Entregáveis:** caso de uso, testes, relatório, mídia da demonstração.
**Depende de:** F0-02, F0-03, F0-04, F0-05
