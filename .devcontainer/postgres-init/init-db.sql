-- Criar usuário zerobudget-owner se não existir
DO $$
BEGIN
    IF NOT EXISTS (SELECT FROM pg_catalog.pg_user WHERE usename = 'zerobudget-owner') THEN
        CREATE USER "zerobudget-owner" WITH PASSWORD 'zerobudget-dev';
    END IF;
END
$$;

-- Criar banco de dados zerobudget se não existir
SELECT 'CREATE DATABASE zerobudget OWNER "zerobudget-owner"'
WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = 'zerobudget')\gexec

-- Conectar ao banco zerobudget e configurar permissões
\c zerobudget

-- Garantir que o usuário zerobudget-owner é o dono do banco
ALTER DATABASE zerobudget OWNER TO "zerobudget-owner";

-- Conceder todas as permissões ao usuário zerobudget-owner
GRANT ALL PRIVILEGES ON DATABASE zerobudget TO "zerobudget-owner";

-- Conceder permissões no schema public
GRANT ALL ON SCHEMA public TO "zerobudget-owner";
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON TABLES TO "zerobudget-owner";
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON SEQUENCES TO "zerobudget-owner";
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON FUNCTIONS TO "zerobudget-owner";


