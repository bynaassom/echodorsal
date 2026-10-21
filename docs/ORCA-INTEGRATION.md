# Integração com o Orca — contrato e cuidados

Alvo: **Orca da stablyai** (IDE de agentes em worktrees paralelos), que traz uma CLI `orca` para scriptar o app em execução.
Este documento é um **ponto de partida**: a CLI evolui com o app. A fonte da verdade é sempre o binário instalado.

## Regras

1. Todo acesso passa por `echodorsal-orca`. Nenhum outro crate executa `orca`.
2. **Descobrir, não presumir.** Usar `orca agent-context --json` (lista de comandos com uso, flags e exemplos) e
   `orca skills get orca-cli` / `orca skills get orchestration --full` (guias versionados pelo binário).
3. Sempre `--json`. Parser **tolerante** (ignora campos desconhecidos; enums com `Unknown`).
4. Falha clara quando um comando não existe na versão instalada; nunca tentar variações "criativas".
5. A CLI fala com o **app em execução**: se o Orca estiver fechado, devolver `OrcaError::NotRunning`.

## Verificação de identidade e versão (startup)

Existem outros projetos chamados "orca" (por exemplo `okikorg/orca-cli`, `orca-cli/orca`, `weill-labs/orca`) com comandos diferentes
— inclusive um `orca usage` que **não** pertence ao Orca da stablyai. No startup:

1. Executar `orca status --json`; falhou → `NotRunning` ou binário errado.
2. Executar `orca agent-context --json` e checar a presença dos comandos que o adaptador usa
   (`worktree`, `orchestration`, etc.). Ausência → `UnsupportedOrca` com a lista do que falta.
3. Registrar `result.runtime.appVersion` e comparar com a matriz abaixo. `CliOrca::compatibility().verified_version` indica se a versão foi verificada; consumidores devem exibir aviso no painel (Sistema) para versão fora da matriz, sem falha dura.

O fingerprint de identidade é o contrato JSON de status e o registro exato dos cinco comandos usados com suporte a `--json`; não é atestado criptográfico do executável. O adaptador aceita `ORCA_BIN` como configuração explícita. Não executa escritas nem tenta vincular/criar Runs. `tasks_for_run(&RunId)` e `gates_for_run(&RunId)` usam `--run` para leitura explícita de um Run existente; `tasks()` e `gates()` preservam o contexto do chamador. Erros `run_required` viram `OrcaError::RunRequired`, preservando a distinção entre falta de contexto e comando incompatível.

## Comandos usados (confirmar com `agent-context`)

> F0-05 confirmou `status`, `agent-context`, `worktree ps`, `orchestration task-list` e `gate-list` no registro da CLI 1.4.237. Os demais comandos abaixo continuam como candidatos para fases futuras, sem garantia de compatibilidade.

### Leitura (Fases 0–3)

| Necessidade | Comando (esperado) |
|---|---|
| Estado do app | `orca status --json` |
| Worktrees (modelo comum) | `orca worktree ps --json` |
| Terminais/leitura de saída | `orca terminal list --json`, `orca terminal read --json` |
| Tasks de orquestração | `orca orchestration task-list --json` (filtros como `--status`, `--ready`, `--brief`) |
| Gates pendentes | `orca orchestration gate-list --json` |
| Detalhe de worker | `orca orchestration worker-show`, `worker-read` |
| Eventos | `orca orchestration check --wait --json` (mensagens: `worker_done`, `escalation`, `question`, `decision_gate`, `merge_ready`, `heartbeat`…) |
| Status dos hooks de agente | `orca agent hooks status` |

Os **dois modelos** precisam ser cobertos:
- **Orquestração** (Runs/Tasks/Dispatches). Status de task: `pending`, `ready`, `dispatched`, `completed`, `failed`, `blocked`.
  A orquestração é **experimental** e precisa ser ativada no app (Configurações → Experimental).
- **Worktrees comuns**: status de card (`todo`, `in-progress`, `in-review`, `completed`) e comentário curto de progresso.

### Escrita (Fase 4, atrás da flag `writes`)

| Ação | Comando (esperado) | Proteção |
|---|---|---|
| Criar task | `orca orchestration task-create …` | releitura + confirmação |
| Iniciar worker | `orca orchestration worker-start …` | confirmação + PIN; permissões em modo manual |
| Resolver gate | `orca orchestration gate-resolve …` | repetir a pergunta e a opção escolhida antes de enviar |

Nunca expor comandos de reset/destrutivos (`orchestration reset`, remoção de worktree) por voz.

## Segurança de permissões

O Orca pré-preenche por padrão flags de bypass de permissões nos agentes (por exemplo `--dangerously-skip-permissions`, `--yolo`).
Um comando de voz mal entendido não pode iniciar um worker com isso. Para workers criados pelo echodorsal:
- configurar o app em **modo manual** para esse fluxo (Configurações → Agents → permissões);
- o adaptador recusa iniciar worker se não conseguir confirmar o modo.

## Fixtures

- Script `scripts/capture-orca-fixtures.sh` roda os comandos de leitura contra um Orca real e grava em `fixtures/orca/<versao>/*.json`.
- `--run-id <id>` captura tasks/gates de um Run existente sem vincular contexto; `--from-cache <dir> --cache-run-index 0` reutiliza capturas locais privadas. Não publicar o ID real.
- **Sanitizar** antes de versionar: remover caminhos pessoais, nomes de clientes, tokens e conteúdo de terminal.
- Teste de contrato: cada fixture deve parsear com o leitor tolerante e produzir o domínio esperado.
- Mudou a versão do Orca → capturar novas fixtures, rodar os testes e atualizar a matriz.

## Binário `orca` falso (testes ponta a ponta)

`tests/support/fake-orca/` contém executáveis por modo (para testes paralelos sem alterar o ambiente do processo) e scripts que imitam `orca` e emitem JSON a partir das fixtures, com modos: `ok`, `not-running`,
`slow` (excede o timeout), `bad-json`, `unknown-fields`. O adaptador é testado diretamente com esses executáveis; o servidor poderá usar `ORCA_BIN` na F0-09. O modo `ok` usa tasks reais sanitizadas e gates vazios de Run explícito; campos/estados futuros e gates não vazios do modo `unknown-fields` são sintéticos.

## Uso e limites (relação com o Orca)

O Orca já rastreia uso de agentes (janelas de 5 h, diária e semanal) e exibe na barra de status/popover do app.
**Ainda não foi confirmado** um comando de CLI que exponha esses números. Spike F0-06 decide entre: ler do Orca, ler direto dos agentes
(`agy`, Codex) ou combinar. Não confundir com o `orca usage` de outros projetos.

## Matriz de compatibilidade

| Versão do Orca | Verificada em | Leitura | Orquestração | Observações |
|---|---|---|---|---|
| 1.4.237 | 2026-10-21 | `status --json`, `worktree ps --json`, `agent-context --json` confirmados e parseados | `task-list --run <id> --json` confirmado (45, 14 e 0 tasks em três Runs existentes); `gate-list --run <id> --json` confirmado com listas vazias; sem Run retorna `run_required` | `runtime.appVersion`; `worktree ps` inclui `totalCount`, `truncated`, `hostScope`; `task_title` confirmado; objeto de gate não vazio ainda sem captura real |

## Evidências e limites da F0-05

Ver `docs/spikes/orca-adapter.md`. O adaptador limita cada subprocesso a 3 s e stdout a 4 MiB, descarta stderr e não inclui respostas cruas nos erros. O contador de worktrees vem acompanhado de paginação e cobertura por host; uma página truncada ou host omitido não representa uma contagem global completa. `workspaceStatus` é o estado do card; o campo `status` da CLI descreve atividade e não é usado como estado do card.

## Riscos

| Risco | Mitigação |
|---|---|
| Orquestração experimental muda flags | isolar no adaptador; fixtures por versão; checagem via `agent-context` |
| App fechado ou travado | timeouts de 3 s; resposta degradada; opção `orca open` configurável |
| Nome "orca" ambíguo | verificação de identidade no startup |
| Estado vivo resetado | event log próprio como fonte do histórico |
