#!/bin/bash

# Teste do fluxo OAuth2 Authorization Code com Dex

set -e

DEX_URL="http://localhost:5556"
CLIENT_ID="test-client"
CLIENT_SECRET="test-secret"
REDIRECT_URI="urn:ietf:wg:oauth:2.0:oob"

echo "=== Teste de Fluxo OAuth2 com Dex ==="
echo ""

# Step 1: Obter configuração OIDC
echo "1️⃣  Obtendo descoberta de configuração OIDC..."
CONFIG=$(curl -s "${DEX_URL}/.well-known/openid-configuration")
echo "$CONFIG" | jq .
echo ""

# Extrair endpoints
AUTH_ENDPOINT=$(echo "$CONFIG" | jq -r '.authorization_endpoint')
TOKEN_ENDPOINT=$(echo "$CONFIG" | jq -r '.token_endpoint')
USERINFO_ENDPOINT=$(echo "$CONFIG" | jq -r '.userinfo_endpoint')
JWKS_URI=$(echo "$CONFIG" | jq -r '.jwks_uri')

echo "Authorization Endpoint: $AUTH_ENDPOINT"
echo "Token Endpoint: $TOKEN_ENDPOINT"
echo "UserInfo Endpoint: $USERINFO_ENDPOINT"
echo "JWKS URI: $JWKS_URI"
echo ""

# Step 2: Gerar PKCE
echo "2️⃣  Gerando PKCE challenge..."
CODE_VERIFIER=$(openssl rand -hex 32)
CODE_CHALLENGE=$(echo -n "$CODE_VERIFIER" | shasum -a 256 | cut -d' ' -f1 | xxd -r -p | base64 | tr -d '=' | tr '+/' '-_')

echo "Code Verifier: $CODE_VERIFIER"
echo "Code Challenge: $CODE_CHALLENGE"
echo ""

# Step 3: Gerar URL de autorização
echo "3️⃣  Gerando URL de autorização..."
STATE=$(openssl rand -hex 16)
NONCE=$(openssl rand -hex 16)

AUTH_URL="${AUTH_ENDPOINT}?client_id=${CLIENT_ID}&response_type=code&scope=openid+profile+email&redirect_uri=${REDIRECT_URI}&state=${STATE}&nonce=${NONCE}&code_challenge=${CODE_CHALLENGE}&code_challenge_method=S256"

echo "Abra esta URL no navegador:"
echo "$AUTH_URL"
echo ""

# Step 4: Instruções manuais
echo "4️⃣  Próximos passos manuais:"
echo ""
echo "   a) Abra a URL acima"
echo "   b) Faça login com:"
echo "      Usuário: admin"
echo "      Senha: password"
echo "   c) Você será redirecionado para: urn:ietf:wg:oauth:2.0:oob?code=<CODE>&state=<STATE>"
echo "   d) Copie o valor de 'code' do redirect"
echo ""
echo "5️⃣  Após obter o código, execute:"
echo ""
echo "   export AUTH_CODE='<CODE_DO_REDIRECT>'"
echo "   export CODE_VERIFIER='$CODE_VERIFIER'"
echo ""
echo "   curl -X POST $TOKEN_ENDPOINT \\"
echo "     -H 'Content-Type: application/x-www-form-urlencoded' \\"
echo "     -d \"client_id=$CLIENT_ID\" \\"
echo "     -d \"client_secret=$CLIENT_SECRET\" \\"
echo "     -d \"code=\$AUTH_CODE\" \\"
echo "     -d \"redirect_uri=$REDIRECT_URI\" \\"
echo "     -d \"grant_type=authorization_code\" \\"
echo "     -d \"code_verifier=\$CODE_VERIFIER\""
echo ""
echo "6️⃣  Depois, use o access_token para chamar userinfo:"
echo ""
echo "   export ACCESS_TOKEN='<TOKEN_DO_STEP_5>'"
echo "   curl -H \"Authorization: Bearer \$ACCESS_TOKEN\" $USERINFO_ENDPOINT"
echo ""
