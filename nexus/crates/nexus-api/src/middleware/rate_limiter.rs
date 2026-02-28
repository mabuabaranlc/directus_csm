use actix_web::body::EitherBody;
use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::{Error, HttpResponse};
use futures_util::future::{ok, LocalBoxFuture, Ready};
use governor::clock::DefaultClock;
use governor::state::{InMemoryState, NotKeyed};
use governor::{Quota, RateLimiter as GovernorRateLimiter};
use std::num::NonZeroU32;
use std::sync::Arc;
use std::task::{Context, Poll};

type SharedLimiter = Arc<GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock>>;

/// Rate limiter middleware using the governor crate.
/// Applies a global request rate limit based on env configuration.
/// Mirrors the rate limiter from Directus (api/src/middleware/rate-limiter-global.ts).
pub struct RateLimiterMiddleware {
    limiter: SharedLimiter,
}

impl RateLimiterMiddleware {
    /// Create a new rate limiter with the given requests per second.
    pub fn new(requests_per_second: u32) -> Self {
        let quota = Quota::per_second(
            NonZeroU32::new(requests_per_second).unwrap_or(NonZeroU32::new(50).unwrap()),
        );
        let limiter = Arc::new(GovernorRateLimiter::direct(quota));
        Self { limiter }
    }

    /// Create from environment variables.
    /// Uses RATE_LIMITER_POINTS (default 50) and RATE_LIMITER_DURATION (default 1s).
    pub fn from_env() -> Self {
        let points = nexus_env::env_number_or("RATE_LIMITER_POINTS", 50) as u32;
        Self::new(points)
    }
}

impl<S, B> Transform<S, ServiceRequest> for RateLimiterMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Transform = RateLimiterService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(RateLimiterService {
            service,
            limiter: self.limiter.clone(),
        })
    }
}

pub struct RateLimiterService<S> {
    service: S,
    limiter: SharedLimiter,
}

impl<S, B> Service<ServiceRequest> for RateLimiterService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        if self.limiter.check().is_err() {
            let response = HttpResponse::TooManyRequests()
                .json(serde_json::json!({
                    "errors": [{
                        "message": "Too many requests, please try again later.",
                        "extensions": { "code": "REQUESTS_EXCEEDED" }
                    }]
                }));
            return Box::pin(async move {
                Ok(req.into_response(response).map_into_right_body())
            });
        }

        let fut = self.service.call(req);
        Box::pin(async move {
            let res = fut.await?;
            Ok(res.map_into_left_body())
        })
    }
}
