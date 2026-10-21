# Orca 1.4.237 — captura sanitizada

Captura de 2026-10-21. `status`, `worktree ps`, `agent-context`, `orchestration task-list`
e `orchestration gate-list` foram executados contra a instalação local. O script preserva
somente campos consumidos pelo adaptador e troca IDs, nomes e hosts por valores demonstrativos.
Os 32 registros de worktree e seus estados vieram da resposta real; paths, comentários,
terminais, dados de projeto e metadados privados foram descartados.

`tasks.json` e `gates.json` registram a resposta real `run_required`, sem mensagem ou metadados.
`tasks-run.json` registra a resposta de um Run existente selecionado com `--run`: 45 tasks,
estados preservados, `task_title` e IDs substituídos por valores demonstrativos.
`gates-run.json` registra a resposta vazia real do mesmo Run. O ID do Run foi substituído
por `run-demo`. Nenhum Run foi criado ou vinculado para a captura.
O formato de um gate não vazio continua exercitado somente com dados sintéticos no falso.
`agent-context.json` conserva somente o registro dos cinco comandos usados e as flags JSON/run.
