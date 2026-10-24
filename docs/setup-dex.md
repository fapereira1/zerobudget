# Dex para desenvolvimento local

O Compose executa Dex e PostgreSQL. Requer Docker com Docker Compose.

```bash
docker compose up -d
curl --fail http://localhost:5556/.well-known/openid-configuration
docker compose logs -f dex
```

A configuração efetiva está em `config/dex-config.yaml`. O issuer é
`http://localhost:5556`; os endpoints devem ser obtidos da descoberta OIDC.
Com esse issuer, os caminhos não têm o prefixo `/dex`.

Credenciais **exclusivas de desenvolvimento**:

| Finalidade | Identificação | Segredo |
| --- | --- | --- |
| Usuário mock | `admin` | `password` |
| Cliente OIDC | `rust-app` | `secret-rust-app-key` |
| Cliente manual | `test-client` | `test-secret` |

O cliente `rust-app` permite `http://localhost:3000/api/auth/callback` e os
outros callbacks listados no YAML. O redirect precisa corresponder exatamente
a uma URI registrada. O client secret autentica o cliente no token endpoint;
não é um token de acesso à aplicação e não habilita `client_credentials`.

Para usar o núcleo Rust, configure `AuthConfig` com o issuer acima,
`rust-app`, o segredo local e um callback registrado; habilite explicitamente
`allow_loopback_http`. Consulte o [contrato do crate auth](../crates/auth/README.md)
para armazenamento da tentativa de login e validação do callback.
A API ainda não implementa os endpoints e o armazenamento de sessões.

O script `scripts/test-oauth2.sh` é uma demonstração manual do protocolo.
Ele imprime valores sensíveis para facilitar a inspeção local e não substitui
a validação criptográfica do crate nem deve ser usado em produção.

As variáveis do `.env.example` anterior não são interpoladas pelo YAML atual:
para mudar portas, issuer ou senhas, altere a configuração e o Compose de forma
coerente. Em produção, use HTTPS, segredos externos e um connector real;
`mockPassword` é apenas para desenvolvimento.

```bash
docker compose down
```

Esse comando para os serviços e mantém o volume de dados.

Referências: [aplicações com Dex](https://dexidp.io/docs/guides/using-dex/),
[OAuth2 no Dex](https://dexidp.io/docs/configuration/oauth2/).

## Testar o crate contra o Dex real

Em um terminal, inicie o provedor:

```bash
docker compose up -d
curl --fail http://localhost:5556/.well-known/openid-configuration
```

Depois execute, na raiz do workspace:

```bash
cargo run -p obz-auth --example dex_login
```

Abra a URL impressa, faça login com `admin` / `password` e autorize o acesso.
O exemplo não abre um servidor HTTP: o navegador pode mostrar erro de conexão
no callback. Copie **a URL completa da barra de endereço**, que contém `code`
e `state`, e cole no terminal antes de cinco minutos. A tentativa permanece
na memória desse único processo, sem um armazenamento de sessões compartilhado.

O resultado esperado é `Autenticação validada`, seguido do issuer, subject,
expiração e confirmação das verificações de autorização. Nenhum token é
impresso. A concessão de leitura é apenas uma demonstração local em memória.
O exemplo usa exclusivamente o cliente e o segredo de desenvolvimento do YAML.
Não compartilhe a URL de retorno nem o histórico do terminal com códigos.

Para testar uma rejeição, execute novamente e altere o valor de `state` na
URL colada: deve retornar `InvalidLogin`. Cada execução precisa de um novo
login; códigos são de uso único. `cargo test -p obz-auth` cobre adicionalmente
assinaturas e claims inválidos sem depender do Docker.
