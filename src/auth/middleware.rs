use crate::auth::jwt::Claims;
#[cfg(not(feature = "mock-auth"))]
use crate::auth::jwt::decode_jwt;
use axum::{
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
};
#[cfg(not(feature = "mock-auth"))]
use axum_extra::TypedHeader;
#[cfg(not(feature = "mock-auth"))]
use headers::{Authorization, authorization::Bearer};

#[derive(Clone)]
pub struct AuthUser(pub Claims);

#[cfg(not(feature = "mock-auth"))]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization(bearer)) =
            TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, _state)
                .await
                .map_err(|_| {
                    (
                        StatusCode::UNAUTHORIZED,
                        "Missing or invalid Authorization header",
                    )
                })?;

        let token = bearer.token();
        let claims: Claims = decode_jwt(token)
            .map_err(|_err| (StatusCode::UNAUTHORIZED, "Invalid or expired token"))?
            .claims;

        Ok(AuthUser(claims))
    }
}

#[cfg(feature = "mock-auth")]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let sub = parts
            .headers
            .get("x-mock-user")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("test_user")
            .to_string();

        Ok(AuthUser(Claims {
            sub,
            exp: usize::MAX,
            iat: 0,
        }))
    }
}
