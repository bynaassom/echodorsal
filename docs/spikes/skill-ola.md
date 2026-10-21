# F0-02 — skill olá em pt-BR (relatório parcial)

## Decisão e estado

Preparar o modelo de interação e as respostas localmente. Conforme decisão do
usuário, F0-03 deve estar concluída antes de expor `/alexa`, inclusive em dev.
Não há endpoint, túnel, bypass de validação ou acesso ao Orca nesta entrega.
O spike completo ainda depende de demonstração e medições reais no Echo Pop.

## Implementado e verificável localmente

- Modelo JSON pt-BR com invocação proposta `eco dorsal`, `SaudeSistemaIntent`,
  ajuda, parar, cancelar e fallback.
- Desserialização mínima para comportamento e serialização da resposta Alexa
  versão `1.0`, com SSML estático. Esses modelos não autenticam requisições.
- Abertura diz "Eco dorsal no ar." e abre sessão com reprompt; consulta de saúde
  diz a mesma frase e encerra. Ajuda/fallback oferecem somente a consulta deste
  teste; parar/cancelar encerram; `SessionEndedRequest` retorna resposta sem fala.
- Testes locais cobrem a estrutura JSON, os comportamentos, intent desconhecido,
  request não suportado e intent malformado. Não medem reconhecimento de voz.

`cargo test -p echodorsal-alexa` reproduz os testes sem Alexa real ou rede.

## Evidências e pendências

| Pergunta | Evidência atual | Estado |
|---|---|---|
| pt-BR aceito na conta? | Arquivo local preparado; sem importação/build na conta | Pendente |
| Nome `eco dorsal` aceito? | Nome proposto no modelo; sem aprovação/build na conta | Pendente |
| Reconhecimento de 20 frases por intent? | Nenhuma execução no simulador/Echo Pop nesta preparação | Pendente; 0 frases medidas, taxa não calculada |
| Latência no Echo Pop? | Nenhuma chamada real | Pendente; sem números |
| Slot de texto livre/frase-veículo? | Não incluído no modelo mínimo, sem teste na conta | Pendente |

## Próxima verificação

Após F0-03 e F0-04, importar/buildar o locale na conta de desenvolvimento e testar
"Alexa, abrir eco dorsal" no Echo Pop. Registrar 20 falas reais com intent esperado,
intent reconhecido, resultado e latência observada, agregando acertos por intent.
Não atribuir acertos de NLU a testes locais que apenas recebem intents prontos.
Verificar separadamente `AMAZON.SearchQuery` e frase-veículo pela documentação
vigente e no simulador antes de introduzi-lo em qualquer fluxo de escrita.

## Referências oficiais consultadas

- [Interaction Model Schema](https://developer.amazon.com/en-US/docs/alexa/smapi/interaction-model-schema.html): estrutura `interactionModel.languageModel`.
- [Request and Response JSON Reference](https://developer.amazon.com/en-US/docs/alexa/custom-skills/request-and-response-json-reference.html): envelope versão `1.0`, SSML e reprompt.
- [Request Types Reference](https://developer.amazon.com/en-US/docs/alexa/custom-skills/request-types-reference.html): abertura, intents e fim de sessão sem fala.

Essas referências orientam o formato local; não comprovam aceitação do modelo
na conta nem reconhecimento no dispositivo.
