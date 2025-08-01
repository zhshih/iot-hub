use crate::auth::jwt::{Claims, decode_jwt};
use axum::{
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
};
use axum_extra::TypedHeader;
use headers::{Authorization, authorization::Bearer};

pub struct AuthUser(pub Claims);

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
