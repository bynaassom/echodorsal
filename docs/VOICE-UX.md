# Voz — echodorsal

Guia de experiência de voz para a skill Alexa (pt-BR). A voz é **consulta curta e comando confirmado**, não conversa.

## Restrições da plataforma (a respeitar)

- Cada turno tem ~8 s para responder. Progressive responses evitam silêncio mas **não estendem o prazo**.
- O Echo Pop não tem tela: não há extensão do tempo de escuta.
- Sessão aberta (`shouldEndSession=false` + reprompt) mantém o microfone aberto por poucos segundos, sem wake word. Silêncio encerra a sessão.
- Funções nativas (alarme, música) continuam funcionando: a skill só atua quando invocada.
- A skill roda em modo de desenvolvimento, apenas na sua conta.

**A verificar na Fase 0 (F0-02):** suporte a pt-BR no modelo de interação; aceitação do nome de invocação; comportamento do slot de texto livre.

## Nome de invocação

Nome do projeto: `echodorsal`. Para a voz, preferir **duas palavras reais**: **"eco dorsal"** (reconhecimento mais confiável que uma palavra inventada).

- "Alexa, abrir eco dorsal" — abre sessão.
- "Alexa, pergunte ao eco dorsal como está o projeto checkout" — pergunta direta.

## Regras de resposta

1. **Até 3 frases.** Primeiro a resposta, depois o detalhe essencial.
2. **Nada de ruído técnico:** sem caminhos, hashes, branches, stack traces. Nomes de tarefas encurtados.
3. **Números por extenso quando falados:** "duas tarefas em curso", "setenta e dois por cento".
4. **Tempo relativo:** "há quatorze minutos", "reseta em uma hora e quarenta".
5. **Erros:** dizer o que houve e o que fazer; nunca "houve um problema" genérico.
6. **Escrita sempre relida e confirmada.** Ação sensível exige PIN falado.
7. Vocabulário igual ao do painel: **próximas, em curso, concluídas, falhou, bloqueada, aguardando você**.

## Intents

### Consulta (Fase 1)

| Intent | Exemplos de fala | Slots |
|---|---|---|
| `StatusProjetoIntent` | "como está o projeto {projeto}" / "status do {projeto}" | `projeto` (`PROJETO`, opcional) |
| `EmAndamentoIntent` | "o que está em andamento" / "o que está rodando" | `projeto` |
| `ConcluidasIntent` | "o que foi concluído {periodo}" / "o que terminou hoje" | `periodo` (`AMAZON.DATE`), `projeto` |
| `ProximasIntent` | "quais são as próximas tarefas" / "o que vem a seguir" | `projeto` |
| `GatesPendentesIntent` | "tem algo esperando minha resposta" / "alguma pergunta pendente" | — |
| `SaudeSistemaIntent` | "o orca está aberto" | — |

### Limites (Fase 3)

| Intent | Exemplos | Slots |
|---|---|---|
| `UsoIntent` | "como estão meus limites" / "quanto falta do codex" | `provedor` (`PROVEDOR`, opcional) |

### Escrita (Fase 4, atrás da flag `writes`)

| Intent | Exemplos | Slots |
|---|---|---|
| `CriarTarefaIntent` | "peça ao eco dorsal para criar tarefa {texto}" | `texto` (`AMAZON.SearchQuery`), `projeto` |
| `ResponderGateIntent` | "sim", "não", "aprovar", "recusar" | — (usa `AMAZON.YesIntent`/`AMAZON.NoIntent` no contexto de um gate) |
| `InformarPinIntent` | "meu pin é {pin}" | `pin` (`AMAZON.FOUR_DIGIT_NUMBER`) |

### Sistema

`AMAZON.HelpIntent`, `AMAZON.StopIntent`, `AMAZON.CancelIntent`, `AMAZON.FallbackIntent`.

- O slot de texto livre (`AMAZON.SearchQuery`) normalmente exige uma **frase-veículo** ("criar tarefa {texto}") e não pode ser a utterance inteira — **confirmar na doc vigente**.
- Slot customizado `PROJETO` é **gerado** a partir de `echodorsal.toml` (script em `scripts/`), incluindo sinônimos de fala.
- Fallback: resposta curta com 2 exemplos do que dá para perguntar. Sem LLM no v1.

## Modelos de resposta

Status do projeto (exemplo):
> "No projeto checkout, duas tarefas estão em curso e uma aguarda sua resposta. Nenhuma falhou hoje."

Em andamento:
> "Em curso: auditoria de billing, no Codex, há catorze minutos; e ajuste do rodapé mobile, no Antigravity, há três."

Concluídas (hoje):
> "Hoje foram concluídas quatro tarefas. A última foi a revisão das rotas de pagamento."

Uso:
> "O Codex está em setenta e dois por cento da janela de cinco horas e reseta em uma hora e quarenta."

SSML: usar `say-as` para números/tempo quando necessário e pausas curtas (`<break time="300ms"/>`) entre ideias. Sem SSML ornamental.

## Erros falados

| Situação | Resposta |
|---|---|
| Orca fechado | "O Orca não está aberto. Abra o app no Mac e pergunte de novo." |
| Orquestração desligada | "A orquestração do Orca está desligada. Ative em Configurações, Experimental." |
| Timeout (> 3 s no Orca) | "O Orca demorou para responder. Tente de novo em instantes." |
| Projeto não reconhecido | "Não achei esse projeto. Os que conheço são {lista curta}." |
| Fallback | "Posso dizer o status, o que está em curso, concluído ou a seguir. Por exemplo: como está o projeto checkout." |
| Escrita desabilitada | "A criação de tarefas por voz está desligada." |

## Fluxo de escrita (Fase 4)

1. Usuário dita: "criar tarefa ajustar o rodapé do billing no mobile".
2. Skill relê (texto original ou, se `Refiner` ativo, versão limpa): "Entendi: ajustar o rodapé do billing no mobile, no projeto checkout. Confirmo?"
3. Sim → se a ação for sensível, pede PIN: "Diga seu PIN."
4. Executa `create_task` com permissões em modo manual; responde: "Tarefa criada. Aviso quando um agente começar."
5. Não/silêncio → cancela: "Cancelado. Nada foi criado."
6. Tudo registrado em `voice_audit`.

Ações **sensíveis** (exigem PIN): iniciar worker, mesclar, qualquer comando destrutivo. Criar uma task pendente pode exigir só confirmação.

## Gates por voz

Quando houver gate pendente e a sessão estiver aberta: o Alexa lê a pergunta e as opções (máx. 2 frases) e aceita "sim"/"não".
Gate com mais de duas opções → "A pergunta tem várias opções; veja no painel."

## Sessão e reprompt

- Consultas: encerrar a sessão após responder, a menos que haja gate pendente.
- Quando a sessão ficar aberta, o reprompt é curto: "Mais alguma coisa?"
- Frase de saída: "tchau", "encerrar" → `AMAZON.StopIntent`.

## Testes de voz

- Tabela `fixtures/voice/utterances.pt-BR.json` (frase → intent/slots esperados), rodada no CI como teste de intent.
- Checagem manual na Fase 0/1: 20 frases reais, com anotação de acertos de reconhecimento por intent.
