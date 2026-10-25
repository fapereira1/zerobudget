# Arquitetura Backend — Zero Budget API em Rust

**Data:** Outubro 2026  
**Stack:** Rust + Axum + PostgreSQL + Redis  
**Padrão:** Domain-Driven Design com Cargo Workspace

---

> Atualização da implementação: o crate `shared` foi removido. Conexão PostgreSQL,
> migrações e os adaptadores `PostgresIdentityRepository` e `PostgresCycleRepository`
> ficam em `infrastructure`. `auth` e `cycle` definem contratos sem depender de SQLx;
> `api` compõe os adaptadores. Os exemplos de arquitetura abaixo são o desenho
> original e não uma descrição integral do estado implementado.

## 1. Visão Geral da Arquitetura

```
┌─────────────────────────────────────────────────────────┐
│                   HTTP Clients (Mobile)                 │
└────────────────────┬────────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────────┐
│              API Crate (Axum)                           │
│  • Routers                                              │
│  • Handlers (HTTP layer)                                │
│  • Middleware (Auth, Tracing, CORS)                     │
└────────────┬───────────────────────────┬────────────────┘
             │                           │
    ┌────────▼──────────┐        ┌───────▼──────────┐
    │ Cycle Crate       │        │ Auth Crate       │
    │ • Service         │        │ • JWT validator  │
    │ • Repository      │        │ • Middleware     │
    │ • Handlers        │        │ • Claims parser  │
    └────────┬──────────┘        └──────────────────┘
             │
    ┌────────▼──────────────────────┐
    │ Domain Crate (Core)           │
    │ • Cycle (Aggregate)           │
    │ • Envelope (Entity)           │
    │ • Transaction (Entity)        │
    │ • Money (Value Object)        │
    │ • Errors (Domain Errors)      │
    └────────┬──────────────────────┘
             │
    ┌────────▼──────────────────────┐
    │ Shared Crate                  │
    │ • DB Pool (sqlx)              │
    │ • Redis Client                │
    │ • Pagination                  │
    │ • Config                      │
    └───────────────────────────────┘
             │
    ┌────────▼──────────────────────┐
    │ PostgreSQL                    │
    │ Redis                         │
    └───────────────────────────────┘
```

---

## 2. Estrutura de Pastas (Cargo Workspace)

```
obz/
├── Cargo.toml                          # Workspace root
├── Cargo.lock
│
├── crates/
│   ├── api/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs                 # Entry point
│   │       ├── router.rs               # Route definitions
│   │       ├── state.rs                # AppState
│   │       ├── middleware/
│   │       │   ├── mod.rs
│   │       │   ├── auth.rs             # JWT extraction
│   │       │   ├── tracing.rs          # Request logging
│   │       │   └── error_handler.rs    # Global error handler
│   │       └── handlers/
│   │           ├── mod.rs
│   │           ├── health.rs           # GET /health
│   │           ├── cycles.rs           # POST/GET /cycles
│   │           ├── envelopes.rs        # POST/GET /envelopes
│   │           ├── transactions.rs     # POST/GET /transactions
│   │           └── reallocations.rs    # POST /reallocations
│   │
│   ├── domain/
│   │   ├── Cargo.toml                  # Zero external deps
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── cycle.rs                # Aggregate root
│   │       │   ├── mod.rs
│   │       │   ├── id.rs               # CycleId (value object)
│   │       │   └── status.rs           # CycleStatus enum
│   │       ├── envelope.rs             # Entity
│   │       │   ├── mod.rs
│   │       │   ├── id.rs               # EnvelopeId
│   │       │   └── status.rs
│   │       ├── transaction.rs          # Entity
│   │       │   ├── mod.rs
│   │       │   └── type.rs             # TransactionType enum
│   │       ├── reallocation.rs         # Entity
│   │       │   └── mod.rs
│   │       ├── money.rs                # Value Object
│   │       │   ├── mod.rs
│   │       │   └── currency.rs
│   │       ├── user.rs                 # Value Object (identity)
│   │       │   └── mod.rs
│   │       └── errors.rs               # Domain errors (DomainError enum)
│   │
│   ├── auth/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── jwt.rs                  # JwtValidator, Claims
│   │       ├── middleware.rs           # Auth middleware
│   │       └── errors.rs               # AuthError
│   │
│   ├── cycle/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── service.rs              # CycleService
│   │       │   ├── create_cycle.rs
│   │       │   ├── activate_cycle.rs
│   │       │   ├── plan_envelope.rs
│   │       │   ├── record_transaction.rs
│   │       │   └── reallocate.rs
│   │       ├── repository.rs           # CycleRepository (trait)
│   │       │   ├── mod.rs
│   │       │   └── postgres.rs         # Implementação
│   │       ├── handlers/
│   │       │   ├── mod.rs
│   │       │   ├── create.rs           # POST /cycles
│   │       │   ├── list.rs             # GET /cycles
│   │       │   ├── get.rs              # GET /cycles/:id
│   │       │   └── activate.rs         # POST /cycles/:id/activate
│   │       └── errors.rs               # CycleError
│   │
│   ├── shared/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── db.rs                   # Database pool
│   │       ├── redis.rs                # Redis client
│   │       ├── config.rs               # App config (env vars)
│   │       ├── error.rs                # AppError (wrapper)
│   │       └── pagination.rs           # Pagination helpers
│   │
│   └── worker/
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           └── jobs/
│               ├── cycle_activator.rs
│               ├── envelope_alert.rs
│               └── cycle_reminder.rs
│
├── migrations/
│   ├── 001_init.sql
│   ├── 002_cycles.sql
│   ├── 003_envelopes.sql
│   ├── 004_transactions.sql
│   ├── 005_reallocations.sql
│   └── 006_indexes.sql
│
├── tests/
│   ├── integration/
│   │   ├── cycles.rs
│   │   ├── transactions.rs
│   │   └── helpers.rs
│   └── fixtures/
│       └── seed.sql
│
└── .env.example
```

---

## 3. Padrões de Design Implementados

### 3.1 Domain-Driven Design (DDD)

#### Aggregate Root: `Cycle`

```rust
pub struct Cycle {
    id: CycleId,
    user_id: UserId,
    period_start: DateTime<Utc>,
    period_end: DateTime<Utc>,
    status: CycleStatus,     // planned, active, closed
    total_income: Money,
    envelopes: Vec<Envelope>, // Collection
    created_at: DateTime<Utc>,
}

impl Cycle {
    /// Domain logic — criar ciclo vazio
    pub fn new(user_id: UserId, period_start: DateTime<Utc>) -> Self {
        Self {
            id: CycleId::new(),
            user_id,
            period_start,
            period_end: period_start + Duration::days(30),
            status: CycleStatus::Planned,
            total_income: Money::zero(),
            envelopes: Vec::new(),
            created_at: Utc::now(),
        }
    }

    /// Domain logic — planejar envelope dentro do ciclo
    pub fn plan_envelope(
        &mut self,
        name: String,
        amount: Money,
        is_mandatory: bool,
    ) -> Result<EnvelopeId, DomainError> {
        // Validações de domínio
        if self.status != CycleStatus::Planned {
            return Err(DomainError::InvalidCycleStatus);
        }
        
        if amount.is_negative() {
            return Err(DomainError::InvalidAmount);
        }

        let envelope = Envelope::new(self.id.clone(), name, amount, is_mandatory);
        let id = envelope.id().clone();
        self.envelopes.push(envelope);
        Ok(id)
    }

    /// Domain logic — registrar transação no envelope
    pub fn record_transaction(
        &mut self,
        envelope_id: &EnvelopeId,
        amount: Money,
    ) -> Result<(), DomainError> {
        if self.status != CycleStatus::Active {
            return Err(DomainError::CycleNotActive);
        }

        let envelope = self.envelopes.iter_mut()
            .find(|e| e.id() == envelope_id)
            .ok_or(DomainError::EnvelopeNotFound)?;

        envelope.debit(amount)?;
        Ok(())
    }

    /// Domain logic — realocação consciente
    pub fn reallocate(
        &mut self,
        from_id: &EnvelopeId,
        to_id: &EnvelopeId,
        amount: Money,
    ) -> Result<(), DomainError> {
        if amount.is_negative() || amount.is_zero() {
            return Err(DomainError::InvalidAmount);
        }

        let from = self.envelopes.iter_mut()
            .find(|e| e.id() == from_id)
            .ok_or(DomainError::EnvelopeNotFound)?;

        from.debit(amount)?;

        let to = self.envelopes.iter_mut()
            .find(|e| e.id() == to_id)
            .ok_or(DomainError::EnvelopeNotFound)?;

        to.credit(amount);
        Ok(())
    }
}
```

#### Entity: `Envelope`

```rust
pub struct Envelope {
    id: EnvelopeId,
    cycle_id: CycleId,
    name: String,
    planned_amount: Money,
    spent_amount: Money,      // Computed from transactions ledger
    is_mandatory: bool,
    created_at: DateTime<Utc>,
}

impl Envelope {
    pub fn balance(&self) -> Money {
        self.planned_amount - self.spent_amount
    }

    pub fn debit(&mut self, amount: Money) -> Result<(), DomainError> {
        if self.spent_amount + amount > self.planned_amount {
            return Err(DomainError::EnvelopeOverbudget);
        }
        self.spent_amount += amount;
        Ok(())
    }

    pub fn credit(&mut self, amount: Money) {
        self.spent_amount -= amount;
    }
}
```

#### Value Object: `Money`

```rust
use rust_decimal::Decimal;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Money {
    cents: i64, // Armazenar em centavos para evitar floating point
}

impl Money {
    pub fn from_decimal(value: Decimal) -> Result<Self, DomainError> {
        let cents = (value * Decimal::from(100)).to_i64()
            .ok_or(DomainError::InvalidMoneyValue)?;
        Ok(Money { cents })
    }

    pub fn zero() -> Self {
        Money { cents: 0 }
    }

    pub fn is_negative(&self) -> bool {
        self.cents < 0
    }

    pub fn is_zero(&self) -> bool {
        self.cents == 0
    }

    // Aritmética
    fn add(&self, other: Money) -> Money {
        Money { cents: self.cents + other.cents }
    }

    fn sub(&self, other: Money) -> Money {
        Money { cents: self.cents - other.cents }
    }
}

impl std::ops::Add for Money {
    type Output = Money;
    fn add(self, other: Money) -> Money {
        self.add(other)
    }
}
```

#### Value Object: `UserId`

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UserId(String);

impl UserId {
    pub fn new(id: String) -> Result<Self, DomainError> {
        if id.is_empty() {
            return Err(DomainError::InvalidUserId);
        }
        Ok(UserId(id))
    }

    pub fn from_jwt_sub(sub: &str) -> Result<Self, DomainError> {
        Self::new(sub.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
```

#### Domain Errors

```rust
#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("Cycle not in planned status")]
    InvalidCycleStatus,

    #[error("Amount must be positive")]
    InvalidAmount,

    #[error("Envelope not found")]
    EnvelopeNotFound,

    #[error("Cycle not active")]
    CycleNotActive,

    #[error("Envelope would exceed budget")]
    EnvelopeOverbudget,

    #[error("Invalid user ID")]
    InvalidUserId,

    #[error("Invalid money value")]
    InvalidMoneyValue,
}
```

---

### 3.2 Repository Pattern (Persistência)

#### Trait (Abstração)

```rust
#[async_trait]
pub trait CycleRepository: Send + Sync {
    async fn save(&self, cycle: &Cycle) -> Result<(), RepositoryError>;
    async fn find_by_id(&self, id: &CycleId, user_id: &UserId) -> Result<Cycle, RepositoryError>;
    async fn find_active(&self, user_id: &UserId) -> Result<Option<Cycle>, RepositoryError>;
    async fn list_by_user(&self, user_id: &UserId) -> Result<Vec<Cycle>, RepositoryError>;
}
```

#### Implementação PostgreSQL

```rust
pub struct PostgresCycleRepository {
    pool: sqlx::PgPool,
}

#[async_trait]
impl CycleRepository for PostgresCycleRepository {
    async fn save(&self, cycle: &Cycle) -> Result<(), RepositoryError> {
        sqlx::query!(
            r#"
            INSERT INTO cycles (id, user_id, period_start, period_end, status, total_income)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (id) DO UPDATE SET
                status = $5,
                total_income = $6
            "#,
            cycle.id().to_string(),
            cycle.user_id().as_str(),
            cycle.period_start(),
            cycle.period_end(),
            cycle.status().to_string(),
            cycle.total_income().to_cents(),
        )
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;

        Ok(())
    }

    async fn find_by_id(&self, id: &CycleId, user_id: &UserId) -> Result<Cycle, RepositoryError> {
        let row = sqlx::query!(
            "SELECT id, period_start, period_end, status, total_income FROM cycles WHERE id = $1 AND user_id = $2",
            id.to_string(),
            user_id.as_str()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?
        .ok_or(RepositoryError::NotFound)?;

        // Reconstruct domain object from row
        Ok(Cycle::from_db_row(row, user_id.clone())?)
    }
}
```

---

### 3.3 Service Layer (Lógica de Aplicação)

```rust
pub struct CycleService {
    repository: Arc<dyn CycleRepository>,
    redis: Arc<RedisClient>,
}

impl CycleService {
    pub async fn create_cycle(
        &self,
        user_id: UserId,
        cycle_dto: CreateCycleDTO,
    ) -> Result<CycleResponse, ApplicationError> {
        // Validações de aplicação (não domínio)
        // Lógica de orquestração
        // Persistência
        // Cache invalidation

        let mut cycle = Cycle::new(user_id.clone(), cycle_dto.period_start);
        
        for envelope_plan in cycle_dto.envelopes {
            cycle.plan_envelope(
                envelope_plan.name,
                envelope_plan.amount,
                envelope_plan.is_mandatory,
            )?;
        }

        self.repository.save(&cycle).await?;
        
        // Invalidate cache
        self.redis.invalidate(&format!("cycles:{}", user_id.as_str())).await.ok();

        Ok(CycleResponse::from_domain(cycle))
    }

    pub async fn record_transaction(
        &self,
        user_id: UserId,
        cycle_id: CycleId,
        envelope_id: EnvelopeId,
        amount: Money,
    ) -> Result<TransactionResponse, ApplicationError> {
        // Load cycle from DB
        let mut cycle = self.repository.find_by_id(&cycle_id, &user_id).await?;

        // Domain logic
        cycle.record_transaction(&envelope_id, amount)?;

        // Persist
        self.repository.save(&cycle).await?;

        // Publish event (para workers consumirem)
        self.publish_event(DomainEvent::TransactionRecorded {
            cycle_id,
            envelope_id,
            amount,
        }).await?;

        Ok(TransactionResponse { /* ... */ })
    }
}
```

---

### 3.4 DTO (Data Transfer Objects)

```rust
// Request DTOs
#[derive(Debug, serde::Deserialize)]
pub struct CreateCycleDTO {
    pub period_start: DateTime<Utc>,
    pub envelopes: Vec<EnvelopePlanDTO>,
}

#[derive(Debug, serde::Deserialize)]
pub struct EnvelopePlanDTO {
    pub name: String,
    pub amount: Decimal,
    pub is_mandatory: bool,
}

#[derive(Debug, serde::Deserialize)]
pub struct RecordTransactionDTO {
    pub envelope_id: String,
    pub amount: Decimal,
}

// Response DTOs
#[derive(Debug, serde::Serialize)]
pub struct CycleResponse {
    pub id: String,
    pub status: String,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub total_income: Decimal,
    pub envelopes: Vec<EnvelopeResponse>,
}

impl CycleResponse {
    pub fn from_domain(cycle: Cycle) -> Self {
        Self {
            id: cycle.id().to_string(),
            status: cycle.status().to_string(),
            period_start: cycle.period_start(),
            period_end: cycle.period_end(),
            total_income: cycle.total_income().to_decimal(),
            envelopes: cycle.envelopes()
                .iter()
                .map(EnvelopeResponse::from_domain)
                .collect(),
        }
    }
}
```

---

### 3.5 HTTP Handlers (Axum)

```rust
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use crate::auth::Claims;

/// POST /api/cycles
/// Criar novo ciclo
pub async fn create_cycle(
    State(state): State<AppState>,
    claims: Claims,  // Extracted via middleware
    Json(dto): Json<CreateCycleDTO>,
) -> Result<(StatusCode, Json<CycleResponse>), ApiError> {
    let user_id = UserId::from_jwt_sub(claims.sub.as_str())?;
    let cycle = state.cycle_service.create_cycle(user_id, dto).await?;
    
    Ok((StatusCode::CREATED, Json(cycle)))
}

/// GET /api/cycles/:id
/// Obter ciclo específico
pub async fn get_cycle(
    State(state): State<AppState>,
    claims: Claims,
    Path(cycle_id): Path<String>,
) -> Result<Json<CycleResponse>, ApiError> {
    let user_id = UserId::from_jwt_sub(claims.sub.as_str())?;
    let cycle_id = CycleId::from_str(&cycle_id)?;
    
    let cycle = state.cycle_service.get_cycle(user_id, cycle_id).await?;
    
    Ok(Json(CycleResponse::from_domain(cycle)))
}

/// POST /api/cycles/:cycle_id/transactions
/// Registrar gasto em envelope
pub async fn record_transaction(
    State(state): State<AppState>,
    claims: Claims,
    Path(cycle_id): Path<String>,
    Json(dto): Json<RecordTransactionDTO>,
) -> Result<(StatusCode, Json<TransactionResponse>), ApiError> {
    let user_id = UserId::from_jwt_sub(claims.sub.as_str())?;
    let cycle_id = CycleId::from_str(&cycle_id)?;
    let envelope_id = EnvelopeId::from_str(&dto.envelope_id)?;
    let amount = Money::from_decimal(dto.amount)?;

    let transaction = state.cycle_service.record_transaction(
        user_id,
        cycle_id,
        envelope_id,
        amount,
    ).await?;

    Ok((StatusCode::CREATED, Json(transaction)))
}
```

---

### 3.6 Middleware (JWT + Auth)

```rust
// src/middleware/auth.rs
use axum::{
    extract::Request,
    http::header::AUTHORIZATION,
    middleware::Next,
    response::IntoResponse,
};

pub async fn auth_middleware(
    mut request: Request,
    next: Next,
) -> Result<Response, AuthError> {
    let auth_header = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or(AuthError::MissingToken)?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(AuthError::InvalidFormat)?;

    let claims = JwtValidator::validate(token)?;
    
    request.extensions_mut().insert(claims);
    
    Ok(next.run(request).await)
}
```

---

## 4. Error Handling Centralizado

```rust
// shared/error.rs
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Domain error: {0}")]
    Domain(#[from] DomainError),

    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),

    #[error("Auth error: {0}")]
    Auth(#[from] AuthError),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Internal server error")]
    Internal,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::Domain(e) => (StatusCode::BAD_REQUEST, e.to_string()),
            AppError::Auth(e) => (StatusCode::UNAUTHORIZED, e.to_string()),
            AppError::Repository(RepositoryError::NotFound) => (
                StatusCode::NOT_FOUND,
                "Resource not found".to_string(),
            ),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "Internal error".to_string()),
        };

        let body = Json(serde_json::json!({
            "error": error_message,
            "status": status.as_u16(),
        }));

        (status, body).into_response()
    }
}
```

---

## 5. Configuração e Estado da Aplicação

```rust
// api/src/state.rs
use std::sync::Arc;

pub struct AppState {
    pub db_pool: sqlx::PgPool,
    pub redis: Arc<RedisClient>,
    pub cycle_service: Arc<CycleService>,
    pub jwt_validator: Arc<JwtValidator>,
}

impl AppState {
    pub async fn new(config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        let db_pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(config.db_max_connections)
            .connect(&config.database_url)
            .await?;

        let redis = Arc::new(RedisClient::connect(&config.redis_url).await?);

        let jwt_validator = Arc::new(
            JwtValidator::new(&config.dex_issuer).await?
        );

        let cycle_service = Arc::new(CycleService {
            repository: Arc::new(PostgresCycleRepository { pool: db_pool.clone() }),
            redis: redis.clone(),
        });

        Ok(AppState {
            db_pool,
            redis,
            cycle_service,
            jwt_validator,
        })
    }
}
```

---

## 6. Testing Strategy

### Unit Tests (Domain Layer)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cycle_creation() {
        let user_id = UserId::new("user-123".to_string()).unwrap();
        let cycle = Cycle::new(user_id, Utc::now());

        assert_eq!(cycle.status(), CycleStatus::Planned);
        assert!(cycle.envelopes().is_empty());
    }

    #[test]
    fn test_envelope_overbudget_protection() {
        let user_id = UserId::new("user-123".to_string()).unwrap();
        let mut cycle = Cycle::new(user_id, Utc::now());

        let amount = Money::from_decimal(Decimal::from(100)).unwrap();
        let envelope_id = cycle.plan_envelope("Food".to_string(), amount, false).unwrap();

        let overbudget = Money::from_decimal(Decimal::from(150)).unwrap();
        let result = cycle.record_transaction(&envelope_id, overbudget);

        assert!(result.is_err());
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_create_cycle_end_to_end() {
    let app_state = setup_test_app().await;
    let client = TestClient::new(app_state.router);

    let response = client
        .post("/api/cycles")
        .header("Authorization", "Bearer valid_jwt_token")
        .json(&CreateCycleDTO {
            period_start: Utc::now(),
            envelopes: vec![],
        })
        .await;

    assert_eq!(response.status(), StatusCode::CREATED);
    let body: CycleResponse = response.json().await;
    assert_eq!(body.status, "planned");
}
```

---

## 7. Decisões Arquiteturais Chave

| Decisão | Justificativa |
|---------|---------------|
| **Domain crate sem deps externas** | Garante testabilidade e independência de frameworks |
| **Repository trait para DB** | Facilita swaps de DB e testes com mocks |
| **DTO layer** | Desacoplamento entre domínio e API |
| **Ledger para saldo** | Auditabilidade completa e zero race conditions |
| **Type-safe queries (sqlx)** | Validação de SQL em compile-time |
| **Async/await everywhere** | Escala com Tokio runtime |
| **DDD bounded contexts** | Escalabilidade: cada crate é independente |

---

## 8. Roadmap de Implementação

### Fase 1: Core (Semana 1-2)
- [ ] Setup do Cargo Workspace
- [ ] Domain crate (Cycle, Envelope, Money, errors)
- [ ] Database migrations
- [ ] PostgresCycleRepository
- [ ] Unit tests do domínio

### Fase 2: Application (Semana 2-3)
- [ ] CycleService
- [ ] Shared crate (DB pool, config, errors)
- [ ] Auth middleware (integrando Dex)
- [ ] API handlers básicos
- [ ] Integration tests

### Fase 3: Polish (Semana 4)
- [ ] Redis caching
- [ ] Error handling refinado
- [ ] Logging/tracing
- [ ] Performance tuning
- [ ] Documentação

### Fase 4: Worker (Semana 5+)
- [ ] Apalis job queue setup
- [ ] Cycle activator job
- [ ] Notification jobs
- [ ] Scheduled tasks

---

## 9. Dependências Cargo (Principais)

```toml
# Domain (zero deps)
[dependencies]
serde = { version = "1", features = ["derive"] }
thiserror = "1"
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }

# API Crate
axum = "0.7"
tokio = { version = "1", features = ["full"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "trace"] }
sqlx = { version = "0.7", features = ["postgres", "runtime-tokio", "uuid", "chrono"] }
redis = { version = "0.25", features = ["aio", "tokio-comp"] }
jsonwebtoken = "9"
tracing = "0.1"
tracing-subscriber = "0.3"
rust_decimal = { version = "1", features = ["serde"] }

# Testing
tokio-test = "0.4"
sqlx-cli = "0.7"
```

---

## 10. Próximos Passos

1. **Criar scaffold do Cargo Workspace** com estrutura proposta
2. **Implementar domain crate** (Cycle, Envelope, Money)
3. **Setup PostgreSQL** com migrations
4. **Conectar Dex** ao middleware de auth
5. **Implementar primeira rota** (POST /cycles) com testes
6. **Integração com Redis** para cache

Quer que eu comece com qual dos passos?
