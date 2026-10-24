# Próximo passo: sessões compartilhadas de autenticação

Status: proposta técnica; implementação ainda não iniciada.

## Ponto de partida

O login com Dex, callback OIDC, consulta de identidade, logout com CSRF e página
de sessão estão implementados. O armazenamento atual vive na memória de uma
única instância: reiniciar a API encerra sessões e tentativas pendentes.
Os testes de `obz-api` e `obz-auth` passam. Há alterações em andamento no domínio
financeiro, fora do escopo desta entrega.

## Objetivo

Compartilhar sessões e tentativas de login em Redis, mantendo o contrato HTTP
atual e as mesmas garantias de expiração, uso único e revogação. Redis já está
previsto na arquitetura do projeto; nesta etapa será armazenamento de estado de
autenticação, não um cache com fallback para memória.

O escopo continua restrito à autenticação de usuários. Persistência de
permissões, autenticação entre APIs e funcionalidades financeiras serão entregas
posteriores.

## Sequência de implementação

1. **Contrato de armazenamento no auth.** Extrair uma interface assíncrona para
   criar e consumir tentativas, criar e consultar sessões, rotacionar IDs e
   revogar sessões. Manter o backend em memória para testes e desenvolvimento
   explícito. Remover do adaptador HTTP a dependência de um mutex global.
2. **Formato interno de dados.** Definir registros versionados com expiração
   absoluta, identidade verificada e token CSRF. Tentativas guardam state, nonce
   e verificador PKCE apenas no servidor. Não serializar `Instant`: usar um
   prazo absoluto e TTL. A reconstrução de `Identity` deve ocorrer somente no
   caminho de armazenamento confiável; não expor um construtor que aceite
   claims de uma requisição como identidade autenticada.
3. **Backend Redis.** Usar TTL de cinco minutos para tentativas e de até trinta
   minutos para sessões, limitado pela expiração da identidade. Consumir cada
   tentativa atomicamente. Rotação, revogação e controle de capacidade devem
   funcionar entre instâncias, sem depender de locks locais. Armazenar as chaves
   de sessão a partir de um hash do identificador opaco; não registrar payloads,
   IDs, códigos ou segredos em logs.
4. **Infraestrutura local e configuração.** Adicionar Redis ao Compose com
   healthcheck e configurar conexão, timeout, namespace e seleção explícita do
   backend. Se a API rodar no host, publicar a porta Redis somente em loopback.
   Documentar autenticação, rede privada/TLS em produção e limites de memória.
   Preferir `noeviction` para esse estado: capacidade esgotada deve ser um erro
   explícito, sem remover sessões arbitrariamente. Definir a política de
   persistência do Redis separadamente: sobreviver ao reinício da API não é
   garantia de sobrevivência à perda do armazenamento.
5. **Integração e documentação.** Injetar o backend na API e manter cookies,
   proteção CSRF, rotas e respostas atuais. Diferenciar armazenamento
   indisponível (503) de sessão ausente/expirada (401). Falhas do Redis não
   devem habilitar acesso nem ativar fallback silencioso em memória.

## Critérios de conclusão

- Uma sessão criada na instância A é reconhecida na instância B.
- Uma tentativa iniciada em A pode ser concluída em B, com o mesmo cookie do
  navegador e sem alterar a vinculação de state, nonce e PKCE.
- Reiniciar somente a API preserva sessões e tentativas ainda válidas.
- Logout em A revoga a sessão também em B; o próximo acesso retorna 401.
- Dois callbacks concorrentes para a mesma tentativa permitem no máximo uma
  troca do código com Dex. O outro é recusado sem criar sessão.
- Expiração continua sendo verificada pela aplicação e pelo armazenamento;
  nenhuma sessão supera o prazo da identidade, inclusive após reconstrução.
- Redis indisponível ou sem capacidade retorna erro controlado e não concede
  acesso. Registros inválidos ou incompatíveis são rejeitados.
- Limites de tentativas/sessões continuam válidos entre instâncias e liberam
  capacidade após expiração ou revogação.
- A suíte existente continua passando, junto de testes de contrato para os
  dois backends e integração com Redis real, isolada por namespace.
- Login e logout pelo navegador são validados contra o Dex do Compose.

## Depois desta etapa

Persistir a identidade local pelo par `(issuer, subject)` e as concessões de
permissão em um banco da aplicação, separado do armazenamento interno do Dex.
Em seguida, entregar a primeira operação financeira autenticada, com verificação
de propriedade no acesso ao banco. Rate limiting distribuído de login também
deve ser planejado antes de exposição pública.

Referências para implementação: [GETDEL](https://redis.io/docs/latest/commands/getdel/)
permite obter e remover uma chave em uma única operação;
[políticas de memória](https://redis.io/docs/latest/develop/reference/eviction/)
descrevem o comportamento de `noeviction`.
