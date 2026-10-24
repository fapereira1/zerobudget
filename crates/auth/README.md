# obz-auth

Núcleo de autenticação de usuários pelo Dex e autorização do ZeroBudget.
O crate é uma biblioteca independente de Axum, banco de dados e regras financeiras.

## Fluxo de login

1. `DexAuth::discover` verifica a descoberta OIDC e carrega as chaves públicas do provedor configurado.
2. `begin_login` cria a URL de autorização e uma `LoginAttempt` com state, nonce e PKCE S256 aleatórios.
3. O adaptador HTTP guarda a tentativa **no servidor, vinculada à sessão do navegador**, por no máximo cinco minutos, e redireciona para a URL.
4. No callback, o adaptador remove a tentativa atomicamente e chama `finish_login` com state e code recebidos. Não deve selecionar tentativas apenas pelo state fornecido na URL: a sessão iniciadora também precisa corresponder.
5. O crate troca o código com o verificador PKCE e valida assinatura RS256, issuer, audience, expiração, nonce, subject não vazio, emissão futura (tolerância de 60 segundos) e `at_hash`, quando presente.
6. O resultado é uma `Identity` verificada, identificada pelo par `(issuer, subject)`. Access tokens e refresh tokens não são expostos.

```rust,no_run
use obz_auth::{AuthConfig, AuthError, DexAuth, Identity, LoginAttempt};

async fn create_auth(secret: String) -> Result<DexAuth, AuthError> {
    DexAuth::discover(AuthConfig::new(
        "https://dex.example.com".into(),
        "rust-app".into(),
        secret,
        "https://app.example.com/api/auth/callback".into(),
        false,
    )?).await
}

async fn callback(
    auth: &DexAuth,
    browser_bound_attempt: LoginAttempt,
    returned_state: &str,
    code: String,
) -> Result<Identity, AuthError> {
    auth.finish_login(browser_bound_attempt, returned_state, code).await
}
```

URLs exigem HTTPS. Apenas no desenvolvimento, `allow_loopback_http = true`
permite HTTP para `localhost`, `127.0.0.1` e `::1`. A configuração deve vir de
uma fonte confiável do servidor, nunca de parâmetros enviados pelo usuário.
O cliente HTTP tem timeout de dez segundos e não segue redirecionamentos.

## Autorização

`Policy` nega acesso por padrão. Permissões são concedidas por configuração
confiável da aplicação para `(issuer, subject)`, nunca por email ou campos da
requisição. Não há promoção automática a administrador.

```rust
use obz_auth::{AuthError, Identity, Permission, Policy};

fn read_budget(
    policy: &Policy,
    identity: &Identity,
    owner_issuer_from_database: &str,
    owner_subject_from_database: &str,
) -> Result<(), AuthError> {
    policy.require_owner(
        identity,
        Permission::ReadBudget,
        owner_issuer_from_database,
        owner_subject_from_database,
    )
}
```

O adaptador deve carregar o proprietário do recurso do banco, ou filtrar suas
consultas pelo par autenticado. IDs de proprietário enviados pelo cliente não
são prova de propriedade. `require` e `require_owner` rejeitam identidades
expiradas, mesmo com permissão cadastrada.

## Integração HTTP ainda necessária

Este núcleo não cria endpoints, cookies nem sessões de aplicação. A camada API
deve implementar login/callback/logout e armazenamento de sessões com expiração
e revogação, remover tentativas de login após qualquer callback e limitar o
número de tentativas por navegador e por servidor. Cookies de sessão devem usar
`HttpOnly`, `Secure` em produção e `SameSite=Lax`; requisições de alteração
precisam de proteção CSRF. A sessão autenticada não pode ultrapassar
`Identity::expires_at()` sem nova autenticação.

Mapeamento sugerido: falhas de autenticação → 401; `Forbidden` → 403;
indisponibilidade do provedor → 503. Erros não carregam códigos, tokens ou
segredos. Não registrar URL de callback completa nem objetos de protocolo.

As chaves ficam em memória durante a vida de `DexAuth`. Recrie a instância
periodicamente e troque-a atomicamente para acompanhar a rotação de chaves do
Dex; chaves desconhecidas são rejeitadas até a atualização. Ainda não há refresh
automático de JWKS, renovação de sessão, persistência de permissões ou logout
federado.

## Limites desta etapa

Somente usuários, conforme o escopo inicial. Autenticação entre APIs fica para
uma etapa posterior. Um client secret registrado no Dex identifica o cliente
OIDC na troca do código; ele não constitui uma credencial para chamar a API.
Access tokens do Dex são opacos e não devem ser tratados como JWTs da API.
O ID token é validado exclusivamente no callback, e não aceito como bearer em
rotas de negócio. A versão do Dex configurada no Compose não fornece o fluxo
`client_credentials` convencional.

## Verificação

```bash
cargo test -p obz-auth
cargo clippy -p obz-auth --all-targets -- -D warnings
```

Os testes cobrem PKCE, state, prazo do login, configuração TLS, tokens RS256
assinados, issuer/audience/nonce incorretos, expiração, assinatura adulterada,
chave desconhecida, substituição de access token e isolamento de proprietários.
A chave privada em `tests/fixtures/test-only-rsa.pem` é pública e exclusiva dos
testes. Os testes não dependem de Dex ou Docker; o login contra o Dex real ainda
precisa de validação de integração.

Referências: [Dex OIDC](https://dexidp.io/docs/openid-connect/),
[fluxos do Dex](https://dexidp.io/docs/configuration/oauth2/),
[biblioteca openidconnect](https://docs.rs/openidconnect/4.0.1/openidconnect/).
