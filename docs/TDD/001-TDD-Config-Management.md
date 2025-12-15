# TDD - Gestão de configuração

## Contexto

O **ZeroBudget** está no começo de seu desenvolvimento e necessita de um pacote de gestão de configurações para ter flexibilidade de execução em distintos ambientes. 

## Motivo do Projeto

A empresa precisa de um sistema robusto de gestão de configuração para injetar valores e segredos dinâmicamente, através do ambiente de execução. 

A configuração de uma aplicação é tudo o que é provável variar entre deploys (homologação, produção, ambientes de desenvolvimento, etc). Isto inclui: 

- Recursos para a base de dados, Redis, e outros serviços de apoio. 
- Credenciais para serviços externos como Amazon S3, Twitter, OpenAI, etc. 
- Valores por deploy como o nome canônico do host para o deploy.

Aplicações as vezes armazenam as configurações no código em constantes. Isso é uma violação do 12factors e NÃO DEVE SER FEITO. 

## Glossário

| Termo           | Definição                                                                              |
|-----------------|----------------------------------------------------------------------------------------|
| Environment    | Qualquer local onde a aplicação possa/deva ser executada     |
| Variável de Ambiente     | Valor armazenada em local do ambiente |
| Env file      | Arquivo contendo diferentes variáveis de ambiente  |
| Secret          | Valor criptografado e armazenado no ambiente de execução                    |


## Fluxo Técnico 

O pacote config deve fornecer um método de carregamento de configuração chamada `Load` que vai encapsular todo o carregamento de variáveis de diferentes categorias. O método deve retornar `(Config, error)` seguindo as regras de `internal/platform/` definidas em docs/ARCHITECTURE-GUIDELINES.md (não pode fazer panic, não pode fazer wrap de erros, deve retornar apenas root cause errors).

Inicialmente vamos separar os valores das seguintes categorias: 

- Application
- Database 
- Caching
- Messaging

Dentro do pacote config, essa segregação deve ser feita em structs go específicas, podemos aplicação o conceiteo de Segregação de Interface as configurações. 

## Solução

| Tarefa         | Descrição                 | Status
|----------------|--------------------------------------|-----------------------------------------------------------------|
| Setup          | Criar o pacote internal/platform/config com o arquivo config.go e criar as structs que irão segregar cada tipo de configuração                    | DEFINIDO   |
| Criar método público       | Criar método público `Load()` que será usado para carregar o mapeamento das configurações do ambiente, retornando `(Config, error)`   | DEFINIDO
