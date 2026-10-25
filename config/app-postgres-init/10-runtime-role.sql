-- Local development only. Owner/migration credentials must not run the API.
CREATE ROLE zerobudget_runtime LOGIN PASSWORD 'runtime-dev-password'
    NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT;
GRANT CONNECT ON DATABASE zerobudget TO zerobudget_runtime;
