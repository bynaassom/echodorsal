# ADR 0001 — Escopo, stack e identidade

- **Status:** aceito
- **Data:** 2026-10-21

## Contexto

Queremos um assistente de gerenciamento de projetos por voz que use o Echo Pop como interface, rode localmente no Mac e se integre ao Orca
(stablyai). O projeto também é material de aprendizado e portfólio no GitHub. Não há projeto de engenharia reversa específico para o Echo Pop,
e mexer no firmware é arriscado e fora do objetivo. O uso de LLMs deve vir de **assinaturas** via CLIs oficiais, não de chaves de API.

## Decisões

1. **Nome:** `echodorsal` (Echo + dorsal, barbatana da orca). Nome de invocação por voz: "eco dorsal" (a validar na F0-02).
2. **Escopo:** assistente de gerenciamento de projetos (status, tarefas em curso/concluídas/próximas, gates, limites de uso).
   **Sem modo conversa.** Voz para consulta curta e comando confirmado.
3. **Integração com a Alexa:** Custom Skill em modo de desenvolvimento (sem mexer no firmware). Funções nativas permanecem intactas.
4. **Backend:** Rust (`axum`, `tokio`, `serde`, SQLite). Arquitetura hexagonal. Contrato com o Orca isolado em um crate.
5. **Painel:** SSR com Askama + HTMX + SSE; TypeScript apenas para ilhas; assets embutidos no binário.
6. **Apps nativos:** Swift para menu bar (v1) e Touch Bar (v2, Control Strip via API privada), como camadas finas que assinam o stream do servidor.
7. **Identidade visual:** dark-first. Paleta `#161925`, `#23395b`, `#406e8e`, `#8ea8c3`, `#dcf763` + derivados (ver `docs/DESIGN.md`).
   Fontes Work Sans + JetBrains Mono. Ícones: Dinkie Icons (MIT), variante padrão; marca `spouting-whale`.
8. **LLM:** opcional, atrás do trait `Refiner`, desligado por padrão, apenas em fluxos assíncronos. Candidato: `gemini-3.8-flash` (`low`) via `agy`.
   A decisão final depende do spike F0-07.
9. **Fonte de uso/limites:** definida pelo spike F0-06 (ADR 0002). Sem métodos não oficiais.
10. **Segurança:** somente leitura por padrão; escrita atrás da flag `writes` com confirmação e PIN; túnel expõe só `/alexa`; validação completa de requisições Alexa.

## Consequências

- (+) Escopo pequeno e demonstrável por fases; LLM não está no caminho crítico; sem custo de API.
- (+) Boa história de portfólio: Rust, segurança, integração com agentes, UX de voz, design consistente.
- (−) Depende de um app Orca em execução e de uma CLI/orquestração experimental que pode mudar.
- (−) O Mac é o servidor: se dormir, a skill fica indisponível. Mitigação: resposta degradada e monitoramento no painel.
- (−) Touch Bar usa API privada; pode quebrar com atualizações do macOS. Isolada e adiada para a v2.
- (−) O `agy` é fechado, com cotas e modelos sujeitos a mudança; o `Refiner` precisa degradar para `NoopRefiner`.

## Alternativas consideradas

- **Engenharia reversa/root do Echo Pop:** descartada (alto risco, sem benefício sobre a skill).
- **Chat por API (ChatGPT/Gemini):** descartada (fora do foco e do modelo de assinatura).
- **Painel em SPA (React/Leptos/Dioxus):** descartada por ora; HTMX + ilhas entrega mais rápido com boa estética. Reavaliar se o painel crescer.
- **Menu bar/Touch Bar em Rust:** suporte fraco para a Touch Bar; Swift é o caminho natural e mais leve.
- **Nome VoxBridge:** descartado (já usado por vários projetos).
