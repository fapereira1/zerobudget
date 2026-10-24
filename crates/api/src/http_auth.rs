use axum::{
    extract::{FromRequestParts, RawQuery, Request, State},
    http::{header, request::Parts, HeaderMap, HeaderValue, StatusCode},
    middleware::{self, Next},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Json, Router,
};
use obz_auth::{AuthError, DexAuth, Identity, SessionError, SessionStore};
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

#[derive(Clone)]
pub struct AppState {
    auth: Arc<RwLock<Arc<DexAuth>>>,
    store: Arc<Mutex<SessionStore>>,
    origin: Arc<str>,
    secure: bool,
}
impl AppState {
    pub fn new(auth: DexAuth, origin: String, secure_cookies: bool) -> Self {
        Self {
            auth: Arc::new(RwLock::new(Arc::new(auth))),
            store: Arc::new(Mutex::new(SessionStore::default())),
            origin: origin.into(),
            secure: secure_cookies,
        }
    }
    pub async fn replace_auth(&self, auth: DexAuth) {
        *self.auth.write().await = Arc::new(auth);
    }
    pub async fn cleanup(&self) {
        self.store.lock().await.cleanup();
    }
    fn login_cookie(&self) -> &'static str {
        if self.secure {
            "__Host-zb_login"
        } else {
            "zb_login"
        }
    }
    fn session_cookie(&self) -> &'static str {
        if self.secure {
            "__Host-zb_session"
        } else {
            "zb_session"
        }
    }
    fn set_cookie(&self, response: &mut Response, name: &str, value: &str, max_age: u64) {
        let secure = if self.secure { "; Secure" } else { "" };
        // Names are constants and values are generated URL-safe opaque IDs.
        let cookie =
            format!("{name}={value}; Path=/; HttpOnly; SameSite=Lax; Max-Age={max_age}{secure}");
        response.headers_mut().append(
            header::SET_COOKIE,
            HeaderValue::from_str(&cookie).expect("generated cookie is ASCII"),
        );
    }
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/auth/session", get(session_page))
        .route("/auth/session.js", get(session_script))
        .route("/auth/session.css", get(session_styles))
        .route("/health", get(|| async { StatusCode::NO_CONTENT }))
        .route("/api/auth/login", get(login))
        .route("/api/auth/callback", get(callback))
        .route("/api/auth/me", get(me))
        .route("/api/auth/logout", post(logout))
        .with_state(state)
        .layer(middleware::from_fn(security_headers))
}

async fn session_page() -> Response {
    let mut response = Html(include_str!("assets/session.html")).into_response();
    response.headers_mut().insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static("default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'"),
    );
    response
}
async fn session_script() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        include_str!("assets/session.js"),
    )
}
async fn session_styles() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        include_str!("assets/session.css"),
    )
}

async fn security_headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response.headers_mut().insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    response
}

pub struct ApiError(StatusCode, &'static str);
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        #[derive(Serialize)]
        struct Error {
            error: &'static str,
        }
        (self.0, Json(Error { error: self.1 })).into_response()
    }
}
impl From<SessionError> for ApiError {
    fn from(error: SessionError) -> Self {
        match error {
            SessionError::Capacity => Self(
                StatusCode::SERVICE_UNAVAILABLE,
                "authentication capacity reached",
            ),
            SessionError::Invalid => Self(StatusCode::UNAUTHORIZED, "invalid or expired session"),
            SessionError::Csrf => Self(StatusCode::FORBIDDEN, "invalid CSRF token"),
        }
    }
}
impl From<AuthError> for ApiError {
    fn from(error: AuthError) -> Self {
        match error {
            AuthError::Forbidden => Self(StatusCode::FORBIDDEN, "permission denied"),
            AuthError::Provider => Self(
                StatusCode::SERVICE_UNAVAILABLE,
                "identity provider unavailable",
            ),
            AuthError::Configuration => Self(
                StatusCode::INTERNAL_SERVER_ERROR,
                "authentication configuration error",
            ),
            _ => Self(StatusCode::UNAUTHORIZED, "authentication failed"),
        }
    }
}

fn cookie(headers: &HeaderMap, name: &str) -> Result<Option<String>, ApiError> {
    let mut value = None;
    for header in headers.get_all(header::COOKIE) {
        let header = header
            .to_str()
            .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "invalid cookie header"))?;
        for item in header.split(';') {
            if let Some((key, token)) = item.trim().split_once('=') {
                if key == name {
                    if value.is_some()
                        || !(22..=128).contains(&token.len())
                        || !token
                            .bytes()
                            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
                    {
                        return Err(ApiError(StatusCode::BAD_REQUEST, "invalid session cookie"));
                    }
                    value = Some(token.to_owned());
                }
            }
        }
    }
    Ok(value)
}

async fn login(State(state): State<AppState>, headers: HeaderMap) -> Result<Response, ApiError> {
    let previous = cookie(&headers, state.login_cookie())?;
    let auth = state.auth.read().await.clone();
    let (url, attempt) = auth.begin_login();
    let id = state
        .store
        .lock()
        .await
        .start_login(attempt, previous.as_deref())?;
    let mut response = Redirect::to(url.as_str()).into_response();
    state.set_cookie(&mut response, state.login_cookie(), &id, 300);
    Ok(response)
}

fn callback_parameters(query: Option<String>) -> Result<(String, String), ApiError> {
    let parameters: Vec<_> =
        url::form_urlencoded::parse(query.as_deref().unwrap_or_default().as_bytes())
            .into_owned()
            .collect();
    if parameters.iter().any(|(name, _)| name == "error") {
        return Err(ApiError(
            StatusCode::UNAUTHORIZED,
            "login cancelled or rejected",
        ));
    }
    let parameter = |name: &str| {
        let values: Vec<_> = parameters.iter().filter(|(key, _)| key == name).collect();
        match values.as_slice() {
            [(_, value)] if !value.is_empty() => Ok(value.clone()),
            _ => Err(ApiError(
                StatusCode::BAD_REQUEST,
                "invalid callback parameters",
            )),
        }
    };
    Ok((parameter("state")?, parameter("code")?))
}

async fn callback(
    State(state): State<AppState>,
    headers: HeaderMap,
    RawQuery(query): RawQuery,
) -> Response {
    let result = finish_callback(&state, &headers, query).await;
    let mut response = match result {
        Ok(response) => response,
        Err(error) => error.into_response(),
    };
    state.set_cookie(&mut response, state.login_cookie(), "", 0);
    response
}
async fn finish_callback(
    state: &AppState,
    headers: &HeaderMap,
    query: Option<String>,
) -> Result<Response, ApiError> {
    let id = cookie(headers, state.login_cookie())?.ok_or(SessionError::Invalid)?;
    // Remove even if the provider rejected consent or callback parameters are malformed.
    let attempt = state.store.lock().await.take_login(&id)?;
    let (returned_state, code) = callback_parameters(query)?;
    let previous = cookie(headers, state.session_cookie())?;
    let auth = state.auth.read().await.clone();
    let identity = auth.finish_login(attempt, &returned_state, code).await?;
    let (session, max_age) = state
        .store
        .lock()
        .await
        .authenticate(identity, previous.as_deref())?;
    let mut response = Redirect::to("/auth/session").into_response();
    state.set_cookie(&mut response, state.session_cookie(), &session, max_age);
    Ok(response)
}

/// Require this extractor in protected handlers. It never accepts bearer ID tokens.
/// Financial handlers must additionally check application permissions and ownership.
pub struct Authenticated {
    pub identity: Identity,
    csrf: String,
    expires_at: u64,
}
impl FromRequestParts<AppState> for Authenticated {
    type Rejection = ApiError;
    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let id = cookie(&parts.headers, state.session_cookie())?.ok_or(SessionError::Invalid)?;
        let mut store = state.store.lock().await;
        let session = store.get(&id)?;
        if !matches!(
            parts.method,
            axum::http::Method::GET | axum::http::Method::HEAD | axum::http::Method::OPTIONS
        ) {
            verify_origin(&parts.headers, state)?;
            session.verify_csrf(csrf_header(&parts.headers)?)?;
        }
        Ok(Self {
            identity: session.identity().clone(),
            csrf: session.csrf_token().to_owned(),
            expires_at: session.expires_at(),
        })
    }
}
#[derive(Serialize)]
struct Me {
    issuer: String,
    subject: String,
    expires_at: u64,
    csrf_token: String,
}
async fn me(authenticated: Authenticated) -> Json<Me> {
    Json(Me {
        issuer: authenticated.identity.issuer().into(),
        subject: authenticated.identity.subject().into(),
        expires_at: authenticated.expires_at,
        csrf_token: authenticated.csrf,
    })
}
fn verify_origin(headers: &HeaderMap, state: &AppState) -> Result<(), ApiError> {
    let origins: Vec<_> = headers.get_all(header::ORIGIN).iter().collect();
    if origins.len() > 1
        || origins
            .first()
            .is_some_and(|origin| origin.to_str().ok() != Some(state.origin.as_ref()))
    {
        return Err(ApiError(StatusCode::FORBIDDEN, "invalid request origin"));
    }
    Ok(())
}
fn csrf_header(headers: &HeaderMap) -> Result<&str, ApiError> {
    let tokens: Vec<_> = headers.get_all("x-csrf-token").iter().collect();
    match tokens.as_slice() {
        [token] => token.to_str().map_err(|_| SessionError::Csrf.into()),
        _ => Err(SessionError::Csrf.into()),
    }
}

async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Result<Response, ApiError> {
    verify_origin(&headers, &state)?;
    let id = cookie(&headers, state.session_cookie())?.ok_or(SessionError::Invalid)?;
    let csrf = csrf_header(&headers)?;
    state.store.lock().await.logout(&id, csrf)?;
    let mut response = StatusCode::NO_CONTENT.into_response();
    state.set_cookie(&mut response, state.session_cookie(), "", 0);
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, extract::Form};
    use http_body_util::BodyExt;
    use openidconnect::{
        core::{
            CoreIdToken, CoreIdTokenClaims, CoreJsonWebKeySet, CoreJwsSigningAlgorithm,
            CoreRsaPrivateSigningKey,
        },
        AccessToken, Audience, EmptyAdditionalClaims, IssuerUrl, JsonWebKeyId, Nonce,
        PkceCodeChallenge, PkceCodeVerifier, PrivateSigningKey, StandardClaims, SubjectIdentifier,
    };
    use serde_json::{json, Value};
    use std::{
        collections::HashMap,
        sync::atomic::{AtomicUsize, Ordering},
    };
    use tower::ServiceExt;

    #[derive(Clone)]
    struct Provider {
        issuer: String,
        login: Arc<std::sync::Mutex<Option<(String, String)>>>,
        exchanges: Arc<AtomicUsize>,
        lifetime: i64,
    }
    fn key() -> CoreRsaPrivateSigningKey {
        CoreRsaPrivateSigningKey::from_pem(
            include_str!("../../auth/tests/fixtures/test-only-rsa.pem"),
            Some(JsonWebKeyId::new("test-key".into())),
        )
        .unwrap()
    }
    async fn discovery(State(provider): State<Provider>) -> Json<Value> {
        Json(json!({
            "issuer": provider.issuer,
            "authorization_endpoint": format!("{}/auth", provider.issuer),
            "token_endpoint": format!("{}/token", provider.issuer),
            "jwks_uri": format!("{}/keys", provider.issuer),
            "response_types_supported": ["code"],
            "subject_types_supported": ["public"],
            "id_token_signing_alg_values_supported": ["RS256"],
            "token_endpoint_auth_methods_supported": ["client_secret_basic"],
            "code_challenge_methods_supported": ["S256"]
        }))
    }
    async fn keys() -> Json<Value> {
        Json(
            serde_json::to_value(CoreJsonWebKeySet::new(vec![key().as_verification_key()]))
                .unwrap(),
        )
    }
    async fn token(
        State(provider): State<Provider>,
        headers: HeaderMap,
        Form(form): Form<HashMap<String, String>>,
    ) -> Response {
        provider.exchanges.fetch_add(1, Ordering::SeqCst);
        // base64("rust-app:secret")
        assert_eq!(headers[header::AUTHORIZATION], "Basic cnVzdC1hcHA6c2VjcmV0");
        assert_eq!(form["grant_type"], "authorization_code");
        assert_eq!(
            form["redirect_uri"],
            "http://localhost:3000/api/auth/callback"
        );
        if form.get("code").map(String::as_str) != Some("valid-code") {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "invalid_grant"})),
            )
                .into_response();
        }
        let (nonce, challenge) = provider.login.lock().unwrap().take().unwrap();
        assert_eq!(
            PkceCodeChallenge::from_code_verifier_sha256(&PkceCodeVerifier::new(
                form["code_verifier"].clone()
            ))
            .as_str(),
            challenge
        );
        let access = AccessToken::new("test-access".into());
        let now = chrono::Utc::now();
        let claims = CoreIdTokenClaims::new(
            IssuerUrl::new(provider.issuer.clone()).unwrap(),
            vec![Audience::new("rust-app".into())],
            now + chrono::Duration::seconds(provider.lifetime),
            now,
            StandardClaims::new(SubjectIdentifier::new("alice".into())),
            EmptyAdditionalClaims {},
        )
        .set_nonce(Some(Nonce::new(nonce)));
        let token = CoreIdToken::new(
            claims,
            &key(),
            CoreJwsSigningAlgorithm::RsaSsaPkcs1V15Sha256,
            Some(&access),
            None,
        )
        .unwrap();
        Json(json!({"access_token": access.secret(), "token_type": "Bearer", "expires_in": 300, "id_token": token.to_string()})).into_response()
    }
    struct Fixture {
        state: AppState,
        provider: Provider,
        task: tokio::task::JoinHandle<()>,
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            self.task.abort();
        }
    }
    impl Fixture {
        async fn new(secure: bool, lifetime: i64) -> Self {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let provider = Provider {
                issuer: format!("http://127.0.0.1:{}", listener.local_addr().unwrap().port()),
                login: Arc::default(),
                exchanges: Arc::default(),
                lifetime,
            };
            let app = Router::new()
                .route("/.well-known/openid-configuration", get(discovery))
                .route("/keys", get(keys))
                .route("/token", post(token))
                .with_state(provider.clone());
            let task = tokio::spawn(async move {
                axum::serve(listener, app).await.unwrap();
            });
            let auth = DexAuth::discover(
                obz_auth::AuthConfig::new(
                    provider.issuer.clone(),
                    "rust-app".into(),
                    "secret".into(),
                    "http://localhost:3000/api/auth/callback".into(),
                    true,
                )
                .unwrap(),
            )
            .await
            .unwrap();
            let state = AppState::new(auth, "http://localhost:3000".into(), secure);
            Self {
                state,
                provider,
                task,
            }
        }
        async fn request(
            &self,
            method: &str,
            uri: &str,
            cookie: Option<&str>,
            csrf: Option<&str>,
            origin: Option<&str>,
        ) -> Response {
            let mut request = axum::http::Request::builder().method(method).uri(uri);
            if let Some(cookie) = cookie {
                request = request.header(header::COOKIE, cookie);
            }
            if let Some(csrf) = csrf {
                request = request.header("x-csrf-token", csrf);
            }
            if let Some(origin) = origin {
                request = request.header(header::ORIGIN, origin);
            }
            router(self.state.clone())
                .oneshot(request.body(Body::empty()).unwrap())
                .await
                .unwrap()
        }
        async fn begin(&self, previous: Option<&str>) -> (String, String) {
            let response = self
                .request("GET", "/api/auth/login", previous, None, None)
                .await;
            assert_eq!(response.status(), StatusCode::SEE_OTHER);
            let cookie = response_cookie(&response, self.state.login_cookie());
            let url =
                url::Url::parse(response.headers()[header::LOCATION].to_str().unwrap()).unwrap();
            let params: HashMap<_, _> = url.query_pairs().into_owned().collect();
            *self.provider.login.lock().unwrap() =
                Some((params["nonce"].clone(), params["code_challenge"].clone()));
            (
                cookie,
                format!(
                    "/api/auth/callback?code=valid-code&state={}",
                    params["state"]
                ),
            )
        }
        async fn authenticate(&self) -> (String, String) {
            let (login_cookie, callback) = self.begin(None).await;
            let response = self
                .request("GET", &callback, Some(&login_cookie), None, None)
                .await;
            assert_eq!(response.status(), StatusCode::SEE_OTHER);
            assert_eq!(response.headers()[header::LOCATION], "/auth/session");
            let cookie = response_cookie(&response, self.state.session_cookie());
            let response = self
                .request("GET", "/api/auth/me", Some(&cookie), None, None)
                .await;
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
            let data: Value =
                serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                    .unwrap();
            assert_eq!(data["subject"], "alice");
            (cookie, data["csrf_token"].as_str().unwrap().into())
        }
    }
    fn response_cookie(response: &Response, name: &str) -> String {
        response
            .headers()
            .get_all(header::SET_COOKIE)
            .iter()
            .find_map(|value| {
                let value = value.to_str().unwrap();
                value
                    .starts_with(&format!("{name}="))
                    .then(|| value.split(';').next().unwrap().into())
            })
            .unwrap()
    }

    #[tokio::test]
    async fn full_login_csrf_and_logout_flow() {
        let fixture = Fixture::new(false, 300).await;
        assert_eq!(
            fixture
                .request("GET", "/api/auth/me", None, None, None)
                .await
                .status(),
            StatusCode::UNAUTHORIZED
        );
        let (cookie, csrf) = fixture.authenticate().await;
        assert_eq!(
            fixture
                .request("GET", "/api/auth/logout", Some(&cookie), None, None)
                .await
                .status(),
            StatusCode::METHOD_NOT_ALLOWED
        );
        assert_eq!(
            fixture
                .request("POST", "/api/auth/logout", Some(&cookie), None, None)
                .await
                .status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            fixture
                .request(
                    "POST",
                    "/api/auth/logout",
                    Some(&cookie),
                    Some("wrong"),
                    None
                )
                .await
                .status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            fixture
                .request(
                    "POST",
                    "/api/auth/logout",
                    Some(&cookie),
                    Some(&csrf),
                    Some("https://attacker.example")
                )
                .await
                .status(),
            StatusCode::FORBIDDEN
        );
        let response = fixture
            .request(
                "POST",
                "/api/auth/logout",
                Some(&cookie),
                Some(&csrf),
                Some("http://localhost:3000"),
            )
            .await;
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert!(response.headers()[header::SET_COOKIE]
            .to_str()
            .unwrap()
            .contains("Max-Age=0"));
        assert_eq!(
            fixture
                .request("GET", "/api/auth/me", Some(&cookie), None, None)
                .await
                .status(),
            StatusCode::UNAUTHORIZED
        );
    }
    #[tokio::test]
    async fn callback_is_browser_bound_and_single_use() {
        let fixture = Fixture::new(false, 300).await;
        let (cookie, callback) = fixture.begin(None).await;
        assert_eq!(
            fixture
                .request("GET", &callback, None, None, None)
                .await
                .status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(fixture.provider.exchanges.load(Ordering::SeqCst), 0);
        assert_eq!(
            fixture
                .request("GET", &callback, Some(&cookie), None, None)
                .await
                .status(),
            StatusCode::SEE_OTHER
        );
        assert_eq!(
            fixture
                .request("GET", &callback, Some(&cookie), None, None)
                .await
                .status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(fixture.provider.exchanges.load(Ordering::SeqCst), 1);
    }
    #[tokio::test]
    async fn invalid_state_and_malformed_callbacks_consume_attempts_without_token_exchange() {
        let fixture = Fixture::new(false, 300).await;
        for suffix in [
            "state=wrong&code=valid-code",
            "state=first&state=second&code=valid-code",
            "error=access_denied",
            "state=first",
        ] {
            let (cookie, valid_callback) = fixture.begin(None).await;
            let response = fixture
                .request(
                    "GET",
                    &format!("/api/auth/callback?{suffix}"),
                    Some(&cookie),
                    None,
                    None,
                )
                .await;
            assert!(response.status().is_client_error());
            assert!(response.headers()[header::SET_COOKIE]
                .to_str()
                .unwrap()
                .contains("Max-Age=0"));
            assert_eq!(
                fixture
                    .request("GET", &valid_callback, Some(&cookie), None, None)
                    .await
                    .status(),
                StatusCode::UNAUTHORIZED
            );
        }
        assert_eq!(fixture.provider.exchanges.load(Ordering::SeqCst), 0);
    }
    #[tokio::test]
    async fn cookies_have_host_prefix_and_secure_attributes_in_production() {
        let fixture = Fixture::new(true, 300).await;
        let response = fixture
            .request("GET", "/api/auth/login", None, None, None)
            .await;
        let cookie = response.headers()[header::SET_COOKIE].to_str().unwrap();
        for required in [
            "__Host-zb_login=",
            "Path=/",
            "HttpOnly",
            "SameSite=Lax",
            "Secure",
        ] {
            assert!(cookie.contains(required));
        }
        assert!(!cookie.contains("Domain="));
        let (cookie, _) = fixture.authenticate().await;
        assert!(cookie.starts_with("__Host-zb_session="));
    }
    #[tokio::test]
    async fn expired_identity_does_not_create_a_session_and_capacity_is_bounded() {
        let fixture = Fixture::new(false, -300).await;
        let (cookie, callback) = fixture.begin(None).await;
        assert_eq!(
            fixture
                .request("GET", &callback, Some(&cookie), None, None)
                .await
                .status(),
            StatusCode::UNAUTHORIZED
        );
        *fixture.state.store.lock().await = SessionStore::new(1, 1);
        let (cookie, _) = fixture.begin(None).await;
        assert_eq!(
            fixture
                .request("GET", "/api/auth/login", None, None, None)
                .await
                .status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
        assert_eq!(
            fixture
                .request("GET", "/api/auth/login", Some(&cookie), None, None)
                .await
                .status(),
            StatusCode::SEE_OTHER
        );
    }
    #[test]
    fn duplicate_session_cookies_are_rejected() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            HeaderValue::from_static(
                "zb_session=abcdefghijklmnopqrstuv; zb_session=abcdefghijklmnopqrstuv",
            ),
        );
        assert!(cookie(&headers, "zb_session").is_err());
    }
    #[tokio::test]
    async fn protected_extractor_enforces_csrf_for_mutations() {
        let fixture = Fixture::new(false, 300).await;
        let (cookie, csrf) = fixture.authenticate().await;
        let (mut parts, _) = axum::http::Request::builder()
            .method("POST")
            .header(header::COOKIE, &cookie)
            .body(())
            .unwrap()
            .into_parts();
        assert!(
            Authenticated::from_request_parts(&mut parts, &fixture.state)
                .await
                .is_err()
        );
        parts
            .headers
            .insert("x-csrf-token", HeaderValue::from_str(&csrf).unwrap());
        assert!(
            Authenticated::from_request_parts(&mut parts, &fixture.state)
                .await
                .is_ok()
        );
        parts.headers.insert(
            header::ORIGIN,
            HeaderValue::from_static("https://attacker.example"),
        );
        assert!(
            Authenticated::from_request_parts(&mut parts, &fixture.state)
                .await
                .is_err()
        );
    }
    #[tokio::test]
    async fn successful_reauthentication_revokes_previous_browser_session() {
        let fixture = Fixture::new(false, 300).await;
        let (old_session, _) = fixture.authenticate().await;
        let (login_cookie, callback) = fixture.begin(Some(&old_session)).await;
        let cookies = format!("{old_session}; {login_cookie}");
        let response = fixture
            .request("GET", &callback, Some(&cookies), None, None)
            .await;
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        let new_session = response_cookie(&response, fixture.state.session_cookie());
        assert_ne!(old_session, new_session);
        assert_eq!(
            fixture
                .request("GET", "/api/auth/me", Some(&old_session), None, None)
                .await
                .status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            fixture
                .request("GET", "/api/auth/me", Some(&new_session), None, None)
                .await
                .status(),
            StatusCode::OK
        );
    }

    #[tokio::test]
    async fn rejected_authorization_code_never_creates_a_session_or_allows_retry() {
        let fixture = Fixture::new(false, 300).await;
        let (cookie, callback) = fixture.begin(None).await;
        let invalid_callback = callback.replace("valid-code", "invalid-code");
        let response = fixture
            .request("GET", &invalid_callback, Some(&cookie), None, None)
            .await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert!(response
            .headers()
            .get_all(header::SET_COOKIE)
            .iter()
            .all(|value| !value.to_str().unwrap().starts_with("zb_session=")));
        assert_eq!(
            fixture
                .request("GET", &callback, Some(&cookie), None, None)
                .await
                .status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(fixture.provider.exchanges.load(Ordering::SeqCst), 1);
    }
    #[tokio::test]
    async fn session_page_serves_html_with_same_origin_script_and_connections() {
        let fixture = Fixture::new(false, 300).await;
        let response = fixture
            .request("GET", "/auth/session", None, None, None)
            .await;
        assert_eq!(response.status(), StatusCode::OK);
        assert!(response.headers()[header::CONTENT_TYPE]
            .to_str()
            .unwrap()
            .starts_with("text/html"));
        let csp = response.headers()[header::CONTENT_SECURITY_POLICY]
            .to_str()
            .unwrap();
        assert!(csp.contains("connect-src 'self'"));
        assert!(csp.contains("script-src 'self'"));
        assert!(!csp.contains("unsafe-inline"));
        assert!(!csp.contains("unsafe-eval"));
        for (path, content_type) in [
            ("/auth/session.js", "text/javascript"),
            ("/auth/session.css", "text/css"),
        ] {
            let response = fixture.request("GET", path, None, None, None).await;
            assert_eq!(response.status(), StatusCode::OK);
            assert!(response.headers()[header::CONTENT_TYPE]
                .to_str()
                .unwrap()
                .starts_with(content_type));
        }
    }
}
