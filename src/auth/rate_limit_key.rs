use axum::http::{HeaderMap, Request};
use std::net::IpAddr;
use tower_governor::GovernorError;
use tower_governor::key_extractor::{KeyExtractor, PeerIpKeyExtractor};

#[derive(Clone, Hash, Eq, PartialEq, Debug)]
pub enum RateLimitKey {
    User(String),
    Ip(IpAddr),
}

#[derive(Clone)]
pub struct UserOrIpKeyExtractor;

impl KeyExtractor for UserOrIpKeyExtractor {
    type Key = RateLimitKey;

    fn extract<T>(&self, req: &Request<T>) -> Result<Self::Key, GovernorError> {
        match extract_user_id(req.headers()) {
            Some(sub) => Ok(RateLimitKey::User(sub)),
            None => PeerIpKeyExtractor.extract(req).map(RateLimitKey::Ip),
        }
    }
}

#[cfg(not(feature = "mock-auth"))]
fn extract_user_id(headers: &HeaderMap) -> Option<String> {
    use crate::auth::jwt::decode_jwt;
    use axum::http::header::AUTHORIZATION;

    headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .and_then(|token| decode_jwt(token).ok())
        .map(|data| data.claims.sub)
}

// mock-auth builds never drive requests through create_app's middleware
// (integration tests mount route sub-routers directly), and decode_jwt isn't
// even compiled in under this feature -- fall through to the IP key.
#[cfg(feature = "mock-auth")]
fn extract_user_id(_headers: &HeaderMap) -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request;
    use serial_test::serial;

    #[test]
    #[serial(env_vars)]
    fn extract_returns_user_key_for_valid_bearer_token() {
        // decode_jwt (and therefore real JWT-based keying) only exists without
        // mock-auth; this repo's dev-dependency on itself with mock-auth
        // enabled means `cargo test --lib` always builds with it active, so
        // -- matching auth::jwt's own test -- gate the body, not the #[test].
        #[cfg(not(feature = "mock-auth"))]
        {
            use crate::auth::jwt::encode_jwt;
            use crate::domain::ids::UserId;
            use axum::http::header::AUTHORIZATION;

            unsafe {
                std::env::set_var("JWT_SECRET", "test-secret");
            }
            let user_id = UserId::new();
            let token = encode_jwt(user_id).expect("failed to encode token");

            let req = Request::builder()
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .body(())
                .unwrap();

            let key = UserOrIpKeyExtractor.extract(&req).unwrap();
            assert_eq!(key, RateLimitKey::User(user_id.to_string()));
        }
    }

    #[test]
    fn extract_falls_back_to_ip_when_no_auth_header() {
        use axum::extract::ConnectInfo;
        use std::net::SocketAddr;

        let addr: SocketAddr = "127.0.0.1:1234".parse().unwrap();
        let mut req = Request::builder().body(()).unwrap();
        req.extensions_mut().insert(ConnectInfo(addr));

        let key = UserOrIpKeyExtractor.extract(&req).unwrap();
        assert_eq!(key, RateLimitKey::Ip(addr.ip()));
    }
}
