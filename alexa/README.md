# Skill olá (pt-BR)

`interactionModels/custom/pt-BR.json` contém o modelo mínimo para importar no
editor JSON do locale pt-BR da Alexa Developer Console. Nome proposto: `eco dorsal`.
A importação e o build na conta Amazon ainda precisam ser verificados.

O comportamento puro está em `echodorsal-alexa`, sem endpoint HTTP. O intent de
saúde responde apenas "Eco dorsal no ar." e não consulta o Orca. Ajuda e fallback
explicam o teste disponível. Abertura, ajuda e fallback mantêm a sessão aberta com
reprompt; saúde, parar e cancelar encerram a sessão. Fim de sessão não fala.

F0-03 deve validar corpo bruto, assinatura, certificado, timestamp, aplicação e
usuário antes de chamar o comportamento e montar `/alexa`. Só então F0-04 poderá
expor a rota pelo túnel. Não existe flag para desabilitar validação.

Não há manifesto com endpoint fictício, skill ID ou credenciais neste diretório.
Configurar o endpoint real na conta de desenvolvimento só após F0-03/F0-04.

Teste local: `cargo test -p echodorsal-alexa`.
Estado e medições pendentes: [relatório do spike](../docs/spikes/skill-ola.md).
