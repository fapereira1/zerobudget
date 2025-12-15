# TDD - Configuração SQLC

## 1. Contexto e Problema

O ZeroBudget é um sistema de controle de orçamento base zero, com backend em Go e banco de dados PostgreSQL.

A aplicação precisa:
- Evoluir o schema de banco de forma segura e auditável.
- Manter o código de acesso a dados alinhado com o schema real.
- Suportar ambientes diferentes (dev, staging, prod) com o mesmo processo.
- Permitir rollback previsível em caso de falhas de deploy.

Hoje não há um padrão consolidado para:
- Onde ficam e como são versionadas as migrations de schema.
- Como gerar e atualizar o código SQL type-safe (queries, models) no Go.
- Como integrar o fluxo de migrations e geração de código na rotina de desenvolvimento e no CI/CD.

## 2. Objetivo do Documento

Definir a solução técnica para:

1. Gerenciar o schema Postgres com migrations versionadas, usando a ferramenta migrate.
2. Gerar código Go type-safe a partir de SQL, usando sqlc.
3. Padronizar a estrutura de diretórios, comandos e integração com CI/CD, para que qualquer desenvolvedor consiga:
Criar e aplicar migrations.
- Gerar/regerar o código do sqlc.
- Rodar a aplicação com o schema correto em qualquer ambiente.
- Este TDD cobre apenas o backend Go e o banco Postgres do ZeroBudget.

