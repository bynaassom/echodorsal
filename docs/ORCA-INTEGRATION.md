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
3. Registrar a versão e comparar com a matriz abaixo. Versão fora da matriz → aviso no painel (Sistema), não falha dura.

## Comandos usados (confirmar com `agent-context`)

> Origem: documentação oficial do Orca e uma wiki de terceiros (versão 1.4.154). **Confirmar cada um na sua versão** na tarefa F0-05.

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
- **Sanitizar** antes de versionar: remover caminhos pessoais, nomes de clientes, tokens e conteúdo de terminal.
- Teste de contrato: cada fixture deve parsear com o leitor tolerante e produzir o domínio esperado.
- Mudou a versão do Orca → capturar novas fixtures, rodar os testes e atualizar a matriz.

## Binário `orca` falso (testes ponta a ponta)

`tests/support/fake-orca/` contém scripts que imitam `orca` e emitem JSON a partir das fixtures, com modos: `ok`, `not-running`,
`slow` (excede o timeout), `bad-json`, `unknown-fields`. O servidor é testado apontando `ORCA_BIN` para esse falso.

## Uso e limites (relação com o Orca)

O Orca já rastreia uso de agentes (janelas de 5 h, diária e semanal) e exibe na barra de status/popover do app.
**Ainda não foi confirmado** um comando de CLI que exponha esses números. Spike F0-06 decide entre: ler do Orca, ler direto dos agentes
(`agy`, Codex) ou combinar. Não confundir com o `orca usage` de outros projetos.

## Matriz de compatibilidade

| Versão do Orca | Verificada em | Leitura | Orquestração | Observações |
|---|---|---|---|---|
| _a preencher na F0-05_ | | | | |

## Riscos

| Risco | Mitigação |
|---|---|
| Orquestração experimental muda flags | isolar no adaptador; fixtures por versão; checagem via `agent-context` |
| App fechado ou travado | timeouts de 3 s; resposta degradada; opção `orca open` configurável |
| Nome "orca" ambíguo | verificação de identidade no startup |
| Estado vivo resetado | event log próprio como fonte do histórico |
