# F0-05 — Adaptador Orca de leitura

## Decisão

Usar exclusivamente a superfície descoberta via `orca agent-context --json` e o guia
versionado `orca skills get orca-cli`. `CliOrca::connect` valida o status do runtime e
os cinco registros com a flag `json` e a flag `run` nos comandos de orquestração. A versão vem de `result.runtime.appVersion`.
O adaptador não expõe argumentos livres ou escrita. O timeout é de 3 s por subprocesso,
com encerramento do filho ao cancelar e limite de stdout de 4 MiB. Erros não carregam
stdout, stderr, paths, mensagens do runtime ou nomes de projetos.

Dependências novas: `serde`/`serde_json` para o contrato tolerante, `thiserror` para erros
tipados e `tokio` com process/time/io-util/rt/macros para subprocessos assíncronos e
cancelamento. São as tecnologias já previstas nas guidelines; sem novo serviço ou framework.

## Evidências — Orca 1.4.237, 2026-10-21

| Chamada | Resultado observado |
|---|---|
| `status --json` | Envelope `{ok,result}`; app ativo, runtime pronto/alcançável, versão 1.4.237 |
| `agent-context --json` | Registro local fora de envelope; os cinco comandos aceitam `--json` |
| `worktree ps --json` | 32 registros, `totalCount=32`, `truncated=false`, host local sem omissões |
| `orchestration task-list --json` | Envelope de erro `run_required` |
| `orchestration gate-list --json` | Envelope de erro `run_required` |
| `orchestration task-list --run <id> --json` | Sucesso em três Runs existentes: 45, 14 e 0 tasks |
| `orchestration gate-list --run <id> --json` | Sucesso nos mesmos Runs: listas vazias |

A descoberta adicional somente leitura identificou Runs já existentes. A flag `--run` permite
consultá-los sem vincular o chamador ou criar estado. `tasks_for_run(&RunId)` e
`gates_for_run(&RunId)` usam argumentos fixos; os métodos sem Run continuam retornando
`RunRequired` quando não há contexto. O fixture real de sucesso preserva 45 tasks e seus
estados; títulos (`task_title`), IDs e o ID do Run são remapeados. `display_name`, spec,
resultados e metadados do processo são descartados. Gates reais foram vazios nos três Runs;
o esquema de um objeto de gate não vazio ainda é exercitado somente sinteticamente.

`worktree ps` tem contagem e cobertura próprias; o retorno preserva esses metadados.
O estado do card vem de `workspaceStatus`, que pode ser nulo; `status=active` não é um card.
IDs são tipos distintos. Campos novos são ignorados e valores novos de enum são preservados.

## Captura e reprodução

`./scripts/capture-orca-fixtures.sh` executa apenas os cinco comandos acima, com timeout.
Não grava respostas brutas: somente campos em allowlist, com nomes, IDs e hosts remapeados.
`--from-cache <diretório>` aplica a mesma sanitização sobre capturas locais privadas.
Os fixtures em `fixtures/orca/1.4.237/` são projeções sanitizadas de respostas reais.
Use `--run-id <id>` para captura explícita e `--cache-run-index N` para selecionar capturas locais `orca-run-N-tasks/gates.json`. O script valida que a resposta pertence ao Run pedido e publica somente o ID demonstrativo. Um gate não vazio exige revisão de schema antes de captura.

`cargo test -p echodorsal-orca` cobre contrato, sucesso, app fechado, timeout, JSON inválido,
campos/estados futuros e campos obrigatórios ausentes. `cargo run -p echodorsal-orca --example probe`
compara o adaptador ao Orca real imprimindo somente versão e contagens. `ORCA_RUN_ID` opcional seleciona um Run existente para tasks/gates, sem imprimir seu ID.

## Limites

O fingerprint detecta CLIs incompatíveis por contrato; não autentica a origem do binário.
Uma versão nova compatível continua utilizável, com `verified_version=false` para aviso do consumidor.
Falha de execução, erro desconhecido do runtime e falta de contexto têm variantes distintas.
O limite total de startup pode chegar a 6 s (duas chamadas); isso ocorre no startup,
nunca dentro de um turno síncrono de voz. Não há polling, event loop ou cache nesta tarefa.

## Validação executada

`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace` (10 testes), `cargo deny check` e instalação/build do web passaram.
Cargo/pnpm usaram caches locais ao projeto e modo offline; a checagem de advisories usou
a base já disponível, sem atualização de rede. `cargo deny` emitiu somente avisos de
licenças permitidas que não aparecem nas dependências atuais.

Sonda real do adaptador com Run explícito: Orca 1.4.237 verificado, 34 worktrees retornadas
de 34, sem truncamento ou hosts omitidos; 45 tasks e 0 gates. O número de worktrees mudou
entre a captura inicial (32) e a sonda (34), refletindo o estado vivo do aplicativo.
Nenhuma criação ou vinculação de Run foi executada.
