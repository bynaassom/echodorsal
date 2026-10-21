# ADR 0004 — Túnel Cloudflare local com credenciais no Keychain

- **Status:** aceito para a implementação local; ativação externa pendente
- **Data:** 2026-10-21

## Contexto

A skill Alexa precisa alcançar o servidor no Mac por HTTPS, mas `AGENTS.md` exige que apenas `/alexa` seja exposto e que segredos fiquem no Keychain. O fluxo habitual de `cloudflared tunnel login` cria um certificado de conta em `~/.cloudflared`; `tunnel create` cria um arquivo JSON com a credencial do túnel. Esses arquivos contrariam a regra de armazenamento deste projeto.

## Decisão

Usar um Cloudflare Tunnel nomeado, com `config_src: local`, para `alexa.bynaassom.com.br`. O arquivo de ingress versionado encaminha somente o caminho exato `^/alexa$` para `127.0.0.1:8080` e termina com `http_status:404`. O servidor mantém bind obrigatório em loopback.

Criar o túnel e o CNAME pela API da Cloudflare com um token de permissões limitadas a `Account > Cloudflare Tunnel > Edit` e `Zone > DNS > Edit`. O token de provisionamento e a credencial específica do túnel ocupam entradas separadas no Keychain. Na execução, o auxiliar lê a credencial do Keychain e a entrega ao `cloudflared` por stdin anônimo, usando `--credentials-file /dev/fd/0`. O processo remove variáveis de ambiente de token e credenciais herdadas, usa logs `warn` e desativa a atualização automática do binário local.

Antes do POST de criação, o provisionador grava no Keychain uma operação pendente com nome de túnel único e segredo gerado localmente. Se houver falha ambígua, uma nova execução para antes de criar outro túnel; a recuperação é manual. Credenciais finais existentes e qualquer registro DNS no hostname também bloqueiam novo provisionamento.

## Consequências

- A configuração de ingress pode ser testada sem DNS e sem credenciais reais.
- O token de API não precisa estar disponível durante a operação normal do túnel.
- O Mac e o processo `cloudflared` precisam permanecer ativos para a skill funcionar. Repouso e queda de rede interrompem o serviço até a reconexão.
- A conclusão da F0-04 ainda exige provisionamento real, testes externos, demonstração da skill e medições de latência. Ver `docs/spikes/tunel.md`.

## Alternativas consideradas

- `cloudflared tunnel login/create` com os arquivos padrão: recusado porque persiste segredos fora do Keychain.
- Túnel remoto por token de execução: simplificaria o início, mas moveria o ingress de `/alexa` para configuração externa; a configuração local verificável é preferida para esta etapa.
- Expor o servidor diretamente: recusado pela superfície adicional e pelo requisito de manter API e painel em loopback.
