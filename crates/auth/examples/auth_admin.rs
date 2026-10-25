//! Trusted operator CLI. Use a database owner/operator URL, never expose over HTTP.
use obz_auth::{DatabaseConfig, Permission, PostgresIdentityRepository};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let usage = "usage: auth_admin migrate | grant USER_UUID budget:read|budget:write | revoke USER_UUID budget:read|budget:write | enable USER_UUID | disable USER_UUID";
    let command = arguments.first().map(String::as_str).ok_or(usage)?;
    // Validate arguments before connecting or making changes.
    let (user, permission) = match (command, arguments.len()) {
        ("migrate", 1) => (None, None),
        ("grant" | "revoke", 3) => (
            Some(arguments[1].parse::<Uuid>()?),
            Some(arguments[2].parse::<Permission>()?),
        ),
        ("enable" | "disable", 2) => (Some(arguments[1].parse::<Uuid>()?), None),
        _ => return Err(usage.into()),
    };
    let url = std::env::var("DATABASE_ADMIN_URL")
        .map_err(|_| "DATABASE_ADMIN_URL is required for trusted administration")?;
    let local = std::env::var("AUTH_ALLOW_LOCAL_HTTP").as_deref() == Ok("true");
    let repository = PostgresIdentityRepository::connect(DatabaseConfig::new(url, local)).await?;
    match command {
        "migrate" => {
            repository.migrate().await?;
            if let Ok(role) = std::env::var("DATABASE_RUNTIME_ROLE") {
                repository.grant_runtime_role(&role).await?;
            }
        }
        "grant" => repository.grant(user.unwrap(), permission.unwrap()).await?,
        "revoke" => {
            repository
                .revoke(user.unwrap(), permission.unwrap())
                .await?
        }
        "enable" => repository.set_enabled(user.unwrap(), true).await?,
        "disable" => repository.set_enabled(user.unwrap(), false).await?,
        _ => unreachable!(),
    }
    println!("Administrative operation completed: {command}");
    Ok(())
}
