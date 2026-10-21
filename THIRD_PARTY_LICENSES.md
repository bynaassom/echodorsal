# Licenças de terceiros

O crate Alexa usa Serde (MIT OR Apache-2.0) para desserializar requests e serializar respostas
conforme o contrato JSON da Alexa; Serde JSON (MIT OR Apache-2.0) é usado nos testes locais.
As dependências transitivas e suas licenças são verificadas por `cargo deny`.
A interface web usa Vite e TypeScript
como dependências de desenvolvimento; os avisos das dependências transitivas serão auditados pelo
`cargo deny` (Rust) e pelos metadados do pnpm antes de qualquer distribuição.

Ativos planejados para tarefas posteriores, ainda não incluídos neste repositório:

- Dinkie Icons — MIT, atelierAnchor (crédito e texto da licença serão incluídos quando os ativos forem adicionados).
- Work Sans — SIL Open Font License 1.1 (a confirmar junto ao pacote distribuído).
- JetBrains Mono — SIL Open Font License 1.1 (a confirmar junto ao pacote distribuído).
