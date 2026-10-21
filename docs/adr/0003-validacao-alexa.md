# ADR 0003 — Validação de requisições Alexa, cache e bind local

- **Status:** aceito
- **Data:** 2026-10-21

## Contexto

A skill do echodorsal é hospedada como um serviço web HTTPS customizado que recebe chamadas diretas da nuvem da Amazon Alexa. A especificação oficial da Amazon impõe requisitos rigorosos de segurança para atestar autenticidade e integridade:
1. Validação da URL do certificado fornecida no cabeçalho `SignatureCertChainUrl` (esquema HTTPS, porta 443, host `s3.amazonaws.com`, caminho `/echo.api/*`).
2. Validação da cadeia de certificados X.509 até uma autoridade certificadora confiável, validação temporal de validade e verificação de SAN (`echo-api.amazon.com`).
3. Verificação de assinatura digital RSA PKCS#1 v1.5 com SHA-256 (`Signature-256`) calculada sobre o corpo bruto (`raw_body`) da requisição HTTP.
4. Verificação de frescor do `timestamp` do envelope JSON (tolerância de ±150 segundos para evitar ataques de replay).
5. Autorização do chamador através de `applicationId` e `userId`.

Além disso, a Regra 4 de `AGENTS.md` veta qualquer bypass de autenticação, inclusive em desenvolvimento. É necessário limitar downloads de certificados e manter o servidor ligado apenas ao endereço local.

## Decisões

1. **Dependências criptográficas (`rustls-webpki` + `ring` + `webpki-roots`)**:
   - Adotamos `rustls-webpki` (com a feature `ring`) para a validação da cadeia X.509 e verificação da assinatura `RSA_PKCS1_2048_8192_SHA256` sobre o corpo bruto (`raw_body`).
   - Adotamos `webpki-roots` (conjunto oficial de Trust Anchors da Mozilla compilado estaticamente) como raiz exclusiva de confiança. A cadeia enviada na requisição nunca adiciona ou altera as raízes confiáveis aceitas.
   - Adotamos `rustls-pki-types` para manuseio seguro e sem cópias de tipos DER e `base64` para decodificação da assinatura.
   - O crate `rcgen` (com `aws_lc_rs`) é restrito exclusivamente a `[dev-dependencies]`, permitindo sintetizar dinamicamente certificados X.509 e assinaturas válidas em testes determinísticos offline, sem chaves privadas ou dados sensíveis versionados no repositório.
   - O runtime não depende de OpenSSL. `ring` inclui código nativo, portanto não é uma pilha exclusivamente Rust.

2. **Normalização e restrições estritas da URL do certificado e SAN**:
   - A URL informada em `SignatureCertChainUrl` deve obrigatoriamente usar esquema `https`, porta padrão 443, host exato `s3.amazonaws.com` e caminho normalizado com prefixo `/echo.api/`.
   - Rejeição de URLs com credenciais, query parameters, caracteres de controle, barras invertidas ou percent-encoding ambíguo. Fragmentos são descartados antes da busca.
   - O certificado de entidade final (leaf) deve conter impreterivelmente o DNS `echo-api.amazon.com` em sua extensão Subject Alternative Name (SAN).

3. **Cache de certificados com revalidação temporal**:
   - Cadeias de certificados válidas são mantidas em memória em `tokio::sync::Mutex<HashMap<String, CachedChain>>`.
   - Limites defensivos: TTL de 1 hora (`CACHE_TTL = 3600s`) e capacidade máxima de 16 entradas (`MAX_CACHE_ENTRIES`), prevenindo vazamento ou exaustão de memória.
   - O mutex protege somente operações curtas no cache. Downloads ocorrem fora do lock global e um semáforo limita a quatro buscas simultâneas. Requisições adicionais aguardam até quatro segundos e verificam o cache novamente antes de baixar; chamadas concorrentes ainda podem buscar a mesma URL mais de uma vez.
   - Downloads de certificados têm limites rígidos de tamanho (máximo 64 KiB), timeout de conexão de 2 s e timeout total de 3 s, rejeitando redirecionamentos e proxies.
   - **Revalidação em cada hit:** mesmo com a cadeia em cache, a verificação de cadeia (`verify_chain`), restrições de CA e validade das datas (`not_before`/`not_after`) é reexecutada a cada requisição contra o `SystemTime::now()`. Isso impede que certificados expirados durante o TTL sejam aceitos.

4. **Códigos de status HTTP e tratamento de erros (Decisão F0-03)**:
   - **`400 Bad Request`** (com corpo vazio): emitido para qualquer falha de integridade, protocolo ou criptografia — cabeçalhos faltantes/duplicados, corpo bruto > 256 KiB, assinatura > 2048 bytes, URL fora da especificação, download de certificado falho, cadeia/SAN inválida, assinatura criptográfica inválida, timestamp fora da tolerância de ±150 segundos, envelope JSON malformado ou timeout de validação (5 s).
   - **`401 Unauthorized`** (com corpo vazio): emitido estritamente quando a requisição é criptograficamente válida e recente, mas `applicationId` ou `userId` não constam na `Allowlist` configurada no `echodorsal.toml`.
   - Respostas de rejeição nunca retornam corpo útil, descrições internas de erro ou stack traces, garantindo comportamento fail-closed opaco.

5. **Bind local obrigatório (`127.0.0.1`)**:
   - `echodorsal-server` valida na inicialização que `config.server.host == "127.0.0.1"`, abortando imediatamente o processo se configurado para bind externo.
   - O listener TCP faz bind estrito em `Ipv4Addr::LOCALHOST`.
   - A restrição do túnel reverso HTTPS ao caminho `/alexa` será implementada na F0-04. O bind local já impede acesso direto externo ao servidor.

## Consequências

- (+) Controles de autenticação alinhados aos requisitos documentados pela Amazon para Custom Skills como web service.
- (+) Prevenção robusta contra requisições forjadas, certificados não autorizados e ataques de repetição (replay).
- (+) Cache em memória com revalidação temporal elimina downloads redundantes no S3 sem risco de aceitar certificados expirados.
- (+) Ausência de dependência de OpenSSL no runtime.
- (−) Primeira chamada fria incorre em latência de download da cadeia do S3 (mitigada por timeouts agressivos e cache subsequente de 1 h).
- (−) Desenvolvimento local exige túnel externo mapeando `/alexa` (F0-04), já que bypass de validação é expressamente proibido.

## Alternativas consideradas

- **Uso do crate `openssl`:** descartado por exigir dependências C do sistema operacional, aumentar superfície de ataque e dificultar compilação cruzada.
- **Bypass de validação em desenvolvimento (`dev_mode`):** descartado por violar a Regra 4 de `AGENTS.md` e criar risco de exposição acidental de endpoints inseguros.
- **Retornar `401` para todos os erros:** descartado; a especificação da Amazon e a decisão de F0-03 diferenciam erros de integridade/formato da requisição (`400`) de chamadores/skills não autorizados na allowlist (`401`).
- **Cache de certificados sem checagem de data por hit:** descartado por abrir brecha caso um certificado perca validade enquanto estiver em cache dentro da janela de 1 hora.
