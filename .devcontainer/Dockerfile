FROM golang:1.25.3

# Instalar ferramentas essenciais
RUN apt-get update && apt-get install -y \
    git \
    curl \
    vim \
    postgresql-client \
    redis-tools \
    && rm -rf /var/lib/apt/lists/*

# Criar usuário vscode (padrão do dev container)
RUN groupadd --gid 1000 vscode \
    && useradd --uid 1000 --gid vscode --shell /bin/bash --create-home vscode

# Configurar ambiente Go
ENV GOPATH=/go
ENV PATH=$PATH:/usr/local/go/bin:$GOPATH/bin

# Instalar ferramentas Go úteis
RUN go install golang.org/x/tools/gopls@latest \
    && go install github.com/go-delve/delve/cmd/dlv@latest \
    && go install golang.org/x/tools/cmd/goimports@latest

# Mudar para usuário vscode
USER vscode

WORKDIR /workspace


