# Guidelines de desenvolvimento — echodorsal

Padrões obrigatórios para humanos e agentes. Regras de escopo e segurança estão em `AGENTS.md`.

## 1. Escopo e disciplina

- Uma fase por vez (`docs/ROADMAP.md`). Ideia fora do roadmap → backlog, não código.
- **Pronto = demonstrável** por voz, painel ou teste automatizado.
- Preferir a solução mais simples que cumpra o critério de aceite. Sem abstração "para o futuro" sem ADR.

## 2. Rust

- Edição estável atual; `rust-version` fixada no workspace.
- `cargo fmt` e `cargo clippy -- -D warnings` no CI. Lints do workspace em `Cargo.toml` (`[workspace.lints]`).
- Sem `unwrap()`/`expect()`/`panic!` fora de testes. `thiserror` em bibliotecas, `anyhow` apenas em binários.
- `tokio` como runtime; timeouts explícitos em toda chamada externa; sem `block_on` em código async.
- `serde` com **leitor tolerante** para saídas do Orca: ignorar campos desconhecidos, exigir só o mínimo, enums com variante `Unknown(String)`.
- `tracing` desde o início, com redação de segredos. Logs estruturados; nada de `println!`.
- Domínio sem dependências de I/O. Tipos novos (`TaskId`, `GateId`) em vez de `String` solta.
- Dependências: justificar adição; `cargo deny` (licenças permitidas, advisories) e `cargo audit` no CI.

## 3. Testes

| Tipo | O que cobre |
|---|---|
| Unitários | domínio, políticas de limite, mapeamento de status, formatação de respostas |
| Contrato | parsers do Orca contra `fixtures/orca/*.json` gravadas de versões reais |
| Intent | tabela de frases pt-BR → intent/slots esperados |
| Ponto a ponta | servidor + **binário `orca` falso** (scripts que emitem JSON) cobrindo sucesso, Orca fechado, timeout e JSON inesperado |
| Segurança | requisições Alexa inválidas (assinatura, timestamp, `userId`) são rejeitadas |

- Fixtures: capturadas por `scripts/capture-orca-fixtures.sh`, **sanitizadas** (sem caminhos pessoais, nomes de clientes ou segredos).
- Todo bug corrigido ganha um teste de regressão.
- Testes não dependem de rede, de Alexa real ou de um Orca em execução.

## 4. Integração com o Orca

- Todo acesso à CLI passa por `echodorsal-orca`. Nenhum outro crate executa `orca`.
- No startup: verificar versão e *identidade* (há outros projetos chamados "orca"); falhar claro se não for o da stablyai.
- Comandos sempre com `--json`. Novas chamadas exigem: fixture, teste de contrato e linha na matriz de compatibilidade.
- Detalhes: `docs/ORCA-INTEGRATION.md`.

## 5. Voz (resumo; detalhes em `docs/VOICE-UX.md`)

- Máximo 3 frases por resposta. Números e horários em SSML (`say-as`) quando necessário.
- Nunca ler caminhos, hashes, código ou erros técnicos. Erros dizem o que houve e o que fazer.
- Toda **escrita** é relida e confirmada antes de executar. Ação sensível exige PIN falado.
- O Alexa não espera: handler responde de cache; trabalho lento vai para fluxo assíncrono.

## 6. Segurança

- Somente leitura por padrão; escrita atrás da flag `writes`.
- Painel só em `127.0.0.1`; túnel expõe só `/alexa`.
- Validação completa de toda requisição Alexa (ver F0-03). Allowlist de `userId` e `applicationId`.
- Segredos no Keychain. Revisar diffs e fixtures em busca de tokens antes de qualquer commit.
- Workers iniciados por voz: permissões em modo manual; não herdar flags de bypass.

## 7. Frontend (painel)

- HTML pelo fio: Askama (SSR) + HTMX + SSE. **TypeScript só para ilhas** (gráficos, barras de uso, paleta de comandos).
- Sem React/Alpine/outro paradigma. Dois (HTMX + TS) já bastam.
- Assets (CSS, JS, fontes WOFF2, sprite de ícones) embutidos no binário. Nenhuma requisição a CDNs ou Google Fonts.
- Identidade, tokens, acessibilidade e copy: `docs/DESIGN.md`. Cores e fontes só via variáveis CSS.
- Acessibilidade: foco visível, contraste ≥ 4,5:1 para texto, status nunca só por cor, `prefers-reduced-motion` respeitado.

## 8. Apps Swift (menu bar; Touch Bar na v2)

- Camada fina: assinam o stream do servidor e renderizam. Sem lógica de negócio no Swift.
- Sem timers de polling; atualização por evento, agrupada. Meta: CPU ≈ 0 em repouso.
- Ícone da menu bar como *template image* (monocromático); estado por ponto colorido.
- Touch Bar: pacote isolado, feature flag, risco documentado (API privada).

## 9. Documentação e ADRs

- Decisão cara de reverter (stack, contrato, segurança, fonte de uso) → ADR curto em `docs/adr/NNNN-slug.md`
  (Contexto, Decisão, Consequências, Alternativas).
- Spikes terminam em `docs/spikes/<tema>.md` com evidências, números medidos e decisão recomendada.
- README público e docs de usuário em PT-BR e EN quando chegar a Fase 5.
- Licenças de terceiros em `THIRD_PARTY_LICENSES.md` (inclui Dinkie Icons — MIT, atelierAnchor).

## 10. Git, PRs e revisão

- Branch `feat/<id>-<slug>`; commits convencionais; PRs pequenos.
- Descrição do PR: o quê, por quê, como verificar, riscos. Link para a tarefa/ADR.
- Checklist de revisão: critérios de aceite; testes; segurança (escrita? confirmação? log?); fixtures sanitizadas; docs atualizadas.
- Nunca fazer merge com CI vermelho. Nunca "consertar" teste de contrato editando a fixture sem entender a mudança do Orca.

## 11. Uso de agentes de código

Ver `docs/AGENT-WORKFLOW.md`. Resumo: especificação clara por tarefa, uma worktree por tarefa,
revisão humana antes do merge, decision gate para qualquer dúvida de escopo ou ação irreversível.
