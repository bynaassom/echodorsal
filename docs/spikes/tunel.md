# F0-04 — Cloudflare Tunnel para a skill Alexa

## Decisão

Usar um túnel nomeado, gerenciado localmente, no hostname `alexa.bynaassom.com.br`. O ingress em `config/cloudflared.example.yml` encaminha apenas o caminho exato `/alexa` para `http://127.0.0.1:8080`; a regra final responde 404. O servidor Rust também deve continuar vinculado exclusivamente a `127.0.0.1`.

As credenciais do túnel e o token de provisionamento ficam em entradas separadas do Keychain do macOS. O processo de execução lê as credenciais e as entrega ao `cloudflared` por um pipe anônimo (`/dev/fd/0`), sem arquivo de segredo. O token de API serve apenas para provisionar túnel e DNS, não para executar o túnel.

## Evidência local

- `cloudflared` 2026.10.3, instalado em `.cache/bin/` dentro deste worktree.
- `python3 scripts/test-tunnel-ingress.py`: validação de ingress e oito casos de roteamento passaram em 21 de outubro de 2026. `/alexa` e `/alexa?x=1` usam a regra do servidor; `/`, `/api`, variantes de `/alexa` e outro hostname usam 404.
- `python3 scripts/test-provision-tunnel.py -v`: cinco testes simulados passaram (sucesso, credenciais preexistentes, DNS A preexistente, POST ambíguo com bloqueio de repetição, ausência de segredo na saída). Não fazem chamadas reais à Cloudflare nem ao Keychain.
- A leitura de credenciais via `/dev/fd/0` foi verificada com entrada sintética inválida: o `cloudflared` leu o pipe e relatou erro de parsing. Isso confirma o mecanismo de entrada, mas não substitui a conexão com a Cloudflare.
- `bash scripts/setup-tunnel-tools.sh` verificou o SHA-256 do arquivo em cache e recompilou o auxiliar do Keychain com caches e temporários em `.cache/`.
- No terminal do Orca, `.cache/bin/tunnel-keychain get-pending` retornou `item not found` (código 2), como esperado antes de qualquer provisionamento. Nenhum segredo foi criado nesse teste.

## Ativação e recuperação

1. Criar um token de API restrito à conta e à zona, com `Account > Cloudflare Tunnel > Edit` e `Zone > DNS > Edit`. Executar `bash scripts/setup-tunnel-tools.sh` neste worktree. Armazenar o token via entrada padrão do auxiliar do projeto, sem colá-lo em comando, arquivo ou conversa.
2. Com Account ID e Zone ID, executar `python3 scripts/provision-tunnel.py --account-id <ACCOUNT_ID> --zone-id <ZONE_ID>`. O script recusa DNS ou credenciais preexistentes, cria nome único e grava uma operação pendente no Keychain **antes** do primeiro POST. Após criar túnel, credenciais e CNAME, apaga a operação pendente. Os IDs não são segredos.
3. Iniciar o servidor local e `python3 scripts/run-tunnel.py <TUNNEL_UUID>` (UUID retornado pelo provisionamento); confirmar a conexão e configurar o endpoint da skill Alexa para `https://alexa.bynaassom.com.br/alexa`.
4. Se o provisionamento parar depois de registrar uma operação pendente, **não repetir o script imediatamente**. Consultar no painel Cloudflare o nome do túnel pendente e o DNS, concluir manualmente ou remover os recursos parciais, conferir as credenciais finais no Keychain e só então executar `.cache/bin/tunnel-keychain delete-pending`. Se recursos forem removidos, excluir também as credenciais finais correspondentes antes de novo provisionamento. O script impede um segundo POST enquanto houver pendência.
5. Se o Mac dormir ou a rede cair, acordar/reconectar o Mac, reiniciar servidor e túnel e repetir as verificações externas. A skill fica indisponível enquanto o Mac ou o túnel estiver fora do ar.

## Verificações externas pendentes

O token de API ainda não foi criado; portanto o túnel e o DNS não foram provisionados. Faltam um `curl` externo de `/alexa` com assinatura inválida (HTTP 400), um `curl` externo de `/api` e do painel (HTTP 404), a demonstração da skill, medições de latência ponta a ponta e observação do comportamento durante repouso do Mac. Não há números medidos a relatar ainda.

## Referências

- [Configuração de ingress de túnel local](https://developers.cloudflare.com/tunnel/features/locally-managed-tunnels/configuration-file/)
- [Criação de túnel por API e permissões do token](https://developers.cloudflare.com/cloudflare-one/networks/connectors/cloudflare-tunnel/get-started/create-remote-tunnel-api/)
- [Permissões e arquivos de credenciais de túnel local](https://developers.cloudflare.com/tunnel/features/locally-managed-tunnels/tunnel-permissions/)
