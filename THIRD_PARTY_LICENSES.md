# Licenças de terceiros

O crate Alexa usa Serde (MIT OR Apache-2.0) para desserializar requests e serializar respostas
conforme o contrato JSON da Alexa; Serde JSON (MIT OR Apache-2.0) é usado nos testes locais.
O crate `webpki-roots` (usado na validação de certificados TLS das requisições Alexa) é licenciado sob **CDLA-Permissive-2.0** (*Community Data License Agreement – Permissive – Version 2.0*). Essa licença aberta da Linux Foundation rege os certificados raiz derivados do Common CA Database (CCADB) da Mozilla; é permissiva, sem copyleft e sem restrições sobre o software ou resultados que consomem os dados, sendo adequada ao projeto.
As dependências transitivas e suas licenças são verificadas por `cargo deny`.
A interface web usa Vite e TypeScript
como dependências de desenvolvimento; os avisos das dependências transitivas serão auditados pelo
`cargo deny` (Rust) e pelos metadados do pnpm antes de qualquer distribuição.

Ativos planejados para tarefas posteriores, ainda não incluídos neste repositório:

- Dinkie Icons — MIT, atelierAnchor (crédito e texto da licença serão incluídos quando os ativos forem adicionados).
- Work Sans — SIL Open Font License 1.1 (a confirmar junto ao pacote distribuído).
- JetBrains Mono — SIL Open Font License 1.1 (a confirmar junto ao pacote distribuído).
