# Desenvolvimento com agentes — echodorsal

Como orquestrar agentes de código (Codex, Antigravity/`agy`, outros) para construir este projeto usando o próprio Orca.
Os comandos do Orca abaixo são o **fluxo esperado**; confirme as flags com `orca skills get orchestration --full` na sua versão
(a orquestração é experimental e os flags evoluem).

## Princípios

1. **Humano decide escopo e faz o merge.** Agentes implementam; você revisa.
2. **Uma tarefa por worktree**, com especificação fechada e critérios de aceite verificáveis (`docs/tasks/`).
3. **Contexto mínimo e certo:** todo agente lê `AGENTS.md` e apenas os docs que a tarefa cita.
4. **Dúvida = parar e perguntar** (decision gate), nunca supor escopo, segurança ou contrato.
5. **Spikes entregam relatório**, não só código: evidências, números medidos e recomendação.
6. **Cota é recurso:** distribua o trabalho entre agentes conforme o uso restante (ver abaixo).

## Papéis

| Papel | Quem | Responsabilidade |
|---|---|---|
| Coordenador | você (e, se quiser, um agente de planejamento) | quebra fases em tasks, define dependências, decide gates |
| Worker de implementação | Codex (sugestão: código Rust, testes) | implementa uma task na própria worktree |
| Worker de apoio | `agy` com Gemini 3.8 Flash `low`/`medium` (sugestão: docs, fixtures, scripts, relatórios de spike) | tarefas de baixo risco e texto |
| Revisor | outro agente + você | revisa diff contra critérios e `docs/GUIDELINES.md`; você aprova o merge |

A divisão é sugestão, não regra: ajuste conforme qualidade observada e cota restante.

## Antes de começar (uma vez)

1. Criar o repositório e copiar `AGENTS.md` e `docs/` deste pacote.
2. Ativar a orquestração no Orca (Configurações → Experimental) e registrar a CLI.
3. Revisar **Configurações → Agents → permissões**: por padrão o Orca pode pré-preencher flags de bypass nos agentes.
   Para este repositório, prefira o modo manual (o projeto lida com tokens e chama serviços reais).
4. Garantir que nenhum segredo esteja no ambiente das worktrees (o projeto usa Keychain; não há `.env` com chaves).
5. Rodar `orca agent-context --json` e guardar a saída em `docs/spikes/orca-agent-context-<versao>.json` (referência para a F0-05).

## Ciclo por tarefa

```
spec (docs/tasks) → run/task no Orca → worker na worktree → PR → revisão (agente + você) → merge → atualizar docs/roadmap
```

1. **Criar o run** da fase e as tasks com dependências (fluxo esperado):
   ```bash
   orca orchestration run-create --objective "Fase 0 do echodorsal" --json
   orca orchestration task-create --spec "<texto da especificação F0-01>" ...   # uma task por especificação
   ```
   > Cada especificação de `docs/tasks/fase-0.md` vira uma task. Copie o texto da seção (ou salve-o em um arquivo e leia-o).
   > Confirme a sintaxe e as flags exatas na sua versão.
2. **Disparar o worker** em worktree nova, com o agente escolhido:
   ```bash
   orca orchestration worker-start --task <id> --worktree new-child --name f0-01-bootstrap --agent codex ...
   ```
3. **Acompanhar** com `orchestration check --wait` e `task-list`. Mensagens `question`/`escalation`/`decision_gate` voltam para você.
4. **Revisar** o diff: critérios de aceite, testes, segurança (escrita? confirmação? segredos?), docs atualizadas.
5. **Merge** só com CI verde. Depois, marcar a task como concluída e registrar aprendizados (ADR se for decisão cara).

## Template de especificação de task

```markdown
## <ID> — <título>
**Contexto:** por que existe; docs a ler (links relativos).
**Escopo:** o que fazer, em passos objetivos.
**Fora do escopo:** o que NÃO fazer.
**Critérios de aceite:** lista verificável (comando, teste ou demonstração).
**Verificação:** como o revisor confirma.
**Entregáveis:** arquivos/pastas esperados e docs a atualizar.
**Depende de:** IDs.
```

## Prompt base para workers

```
Você está trabalhando no repositório echodorsal.
1. Leia AGENTS.md e os documentos citados na tarefa. Em conflito, AGENTS.md e os ADRs vencem.
2. Implemente SOMENTE o escopo da tarefa. Não antecipe fases futuras.
3. Não invente comandos do Orca: descubra com `orca agent-context --json`.
4. Rode fmt, clippy e testes antes de concluir. Mostre a saída.
5. Se algo for ambíguo, ou envolver segurança, escrita ou mudança de contrato, PARE e abra uma pergunta.
6. Entregue: código, testes, docs atualizados e um resumo curto do que foi feito e do que ficou pendente.
```

## Paralelização da Fase 0

```
F0-01 (bootstrap) ─┬─ F0-02 (skill olá) ── F0-03 (validação) ── F0-04 (túnel) ──┐
                   ├─ F0-05 (OrcaAdapter + fixtures) ───────────────────────────┤
                   ├─ F0-06 (spike de uso) ─────────────────────────────────────┼─ F0-09 (voz → Orca, ponta a ponta)
                   ├─ F0-07 (spike agy headless)                                 │
                   └─ F0-08 (spike ícones/tokens) ───────────────────────────────┘
```

F0-01 vai primeiro e sozinho. Depois F0-02, F0-05, F0-06, F0-07 e F0-08 podem rodar em paralelo.

## Gates humanos recomendados

- Qualquer mudança de contrato com o Orca ou de fixtures.
- Qualquer coisa que toque autenticação, validação Alexa, túnel ou escrita.
- Escolha da fonte de uso (F0-06) e do `Refiner` (F0-07): viram ADR.
- Antes de ligar a flag `writes` pela primeira vez.

## Distribuição de cota entre agentes

- Consulte o uso antes de despachar tarefas grandes (popover de uso do Orca; `agy`/Codex conforme F0-06).
- Tarefas de texto e relatórios → `agy` (`gemini-3.8-flash` `low`/`medium`); implementação e testes complexos → Codex.
- Se uma janela passar de ~80%, adie tarefas não críticas ou redistribua. Isso será automatizado na Fase 3.
- Observação: a cota do `agy` e os modelos disponíveis para assinantes mudaram recentemente; verifique antes de planejar a semana.

## Antipadrões

- Tarefa vaga ("faça a skill"). Sempre com critérios de aceite.
- Várias tarefas na mesma worktree.
- Agente "consertando" fixtures para o teste passar.
- Aceitar PR sem ler o diff porque "o agente testou".
- Dar ao agente contexto demais (todos os docs) em vez do necessário.
- Rodar workers com bypass de permissões neste repositório.
