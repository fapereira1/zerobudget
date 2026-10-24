# ZeroBudget API

Integração HTTP do `obz-auth` com Axum. O login usa Authorization Code com
PKCE no Dex; a aplicação mantém uma sessão própria no servidor. ID tokens e
access tokens do Dex não são aceitos como bearer em rotas de negócio.

## Desenvolvimento local

Na raiz do workspace:

```bash
docker compose up -d
AUTH_ALLOW_LOCAL_HTTP=true \
DEX_CLIENT_SECRET=secret-rust-app-key \
cargo run -p obz-api
```

Abra **http://localhost:3000/api/auth/login** no navegador e entre com
`admin` / `password`. Após autorizar no Dex, a API recebe o callback e
redireciona para `/auth/session`, uma página HTML com a identidade e o botão Sair.
O endpoint `/api/auth/me` continua retornando identidade e token CSRF em JSON.
Não é mais necessário copiar URLs para o terminal.

Use `localhost` durante todo o fluxo. Alternar para `127.0.0.1` muda o host do
cookie e invalida a vinculação da tentativa de login.

| Método | Rota | Comportamento |
| --- | --- | --- |
| GET | `/auth/session` | Página HTML para consultar sessão e sair |
| GET | `/health` | 204, sem autenticação; verifica apenas que a API responde |
| GET | `/api/auth/login` | Cria tentativa de cinco minutos e redireciona para Dex |
| GET | `/api/auth/callback` | Consome a tentativa e cria uma nova sessão após validar o ID token |
| GET | `/api/auth/me` | Identidade verificada, expiração Unix e `csrf_token`; 401 sem sessão |
| POST | `/api/auth/logout` | Exige sessão e `X-CSRF-Token`; revoga a sessão e remove o cookie |

Para testar logout, abra **http://localhost:3000/auth/session** e clique em
**Sair**. A página envia o token CSRF, confirma HTTP 204 e consulta novamente a
sessão para verificar HTTP 401. Não é necessário executar código no console.

O visualizador de JSON do Firefox pode aplicar sua própria CSP e bloquear
`fetch` executado no console de `/api/auth/me`. Use a página HTML acima; não
desative a CSP ou outras proteções do navegador.

Sem o header CSRF, logout retorna 403 e mantém a sessão ativa.
Logout encerra a sessão do ZeroBudget; ele não encerra a sessão do Dex ou do
provedor upstream. Um novo login pode aproveitar o SSO existente no Dex.

## Configuração

| Variável | Padrão | Uso |
| --- | --- | --- |
| `API_BIND` | `127.0.0.1:3000` | Endereço do listener |
| `API_PUBLIC_URL` | `http://localhost:3000` | Origem pública, sem caminho; determina callback e cookies seguros |
| `DEX_ISSUER` | `http://localhost:5556` | Issuer OIDC confiável |
| `DEX_CLIENT_ID` | `rust-app` | Cliente registrado no Dex |
| `DEX_CLIENT_SECRET` | obrigatório | Segredo do cliente; não é registrado em logs |
| `AUTH_ALLOW_LOCAL_HTTP` | `false` | Exceção explícita de desenvolvimento para HTTP em loopback |

A aplicação lê variáveis do ambiente; não carrega `.env` automaticamente.
O callback é sempre `${API_PUBLIC_URL}/api/auth/callback` e precisa estar
registrado no Dex. Em produção, configure origem pública e issuer HTTPS,
forneça o segredo externamente e mantenha `AUTH_ALLOW_LOCAL_HTTP=false`.
Terminação TLS pode ocorrer no proxy; a origem pública configurada determina
`Secure`, sem confiar em headers encaminhados enviados pelo cliente.

## Segurança e armazenamento

- Cookies são host-only, `HttpOnly`, `SameSite=Lax`, com `Path=/`. Em HTTPS,
  recebem `Secure` e nomes com prefixo `__Host-`.
- `state`, `nonce` e o verificador PKCE permanecem na memória do servidor.
  A tentativa é selecionada pelo cookie do navegador, removida atomicamente
  antes da troca do código e invalidada mesmo em callbacks recusados ou malformados.
- A sessão usa um ID opaco aleatório e é rotacionada após cada login válido.
  A sessão anterior do mesmo navegador é revogada quando houver um novo login.
- A duração absoluta máxima é de 30 minutos e nunca ultrapassa a expiração do
  ID token. Não há renovação automática nem cookie persistente além desse limite.
- `Authenticated` protege handlers com uma identidade validada. Para métodos
  que alteram dados, exige CSRF; quando `Origin` é enviado, exige a origem pública
  configurada. Logout aplica as mesmas verificações. CORS não está habilitado.
- Permissões financeiras continuam negadas por padrão. Novos handlers devem
  usar `Policy` e conferir propriedade a partir do banco, além do extractor.
  O login não concede permissões financeiras automaticamente.
- Respostas usam `Cache-Control: no-store`, `Referrer-Policy: no-referrer` e
  `X-Content-Type-Options: nosniff`. Tokens, cookies e URLs de callback não são logados.
- O armazenamento aceita no máximo 1.024 tentativas e 4.096 sessões por processo.
  Entradas expiradas são removidas em acessos e a cada minuto; capacidade esgotada
  retorna 503. O proxy de produção deve também limitar a taxa de login.
- Descoberta e JWKS são atualizados a cada cinco minutos. Uma falha mantém as
  últimas chaves disponíveis; chaves desconhecidas continuam rejeitadas.

O armazenamento é **em memória e por processo**: reiniciar a API encerra sessões
e tentativas. Esta implementação atende uma única instância; persistência e
armazenamento compartilhado para múltiplas réplicas ficam para uma etapa posterior.
Não há logout federado, renovação silenciosa ou autenticação de serviços.

## Testes

```bash
cargo test -p obz-api -p obz-auth
cargo clippy -p obz-api -p obz-auth --all-targets -- -D warnings
```

Os testes HTTP iniciam um provedor OIDC temporário em loopback, com descoberta,
JWKS, tokens RSA assinados e verificação de PKCE no endpoint de troca. Verificam
login/callback/me/logout, cookies seguros, vinculação ao navegador, callback de
uso único, callbacks inválidos, CSRF, origem, expiração e limite de capacidade.
Não exigem Docker; exigem permissão para abrir portas locais temporárias.

Referências: [Axum](https://docs.rs/axum/0.8.9/axum/),
[OWASP: sessões](https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html),
[OWASP: CSRF](https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html).
