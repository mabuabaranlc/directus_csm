use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::Error;
use futures_util::future::{ok, LocalBoxFuture, Ready};
use std::rc::Rc;

/// Security headers middleware — equivalent to helmet in Express
/// Mirrors api/src/middleware/respond.ts + helmet configuration
pub struct SecurityHeaders;

impl<S, B> Transform<S, ServiceRequest> for SecurityHeaders
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = SecurityHeadersMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(SecurityHeadersMiddleware {
            service: Rc::new(service),
        })
    }
}

pub struct SecurityHeadersMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for SecurityHeadersMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();

        Box::pin(async move {
            let mut res = service.call(req).await?;
            let headers = res.headers_mut();

            use actix_web::http::header::HeaderValue;

            // Content Security Policy
            let csp = nexus_env::env_string_or(
                "CONTENT_SECURITY_POLICY",
                "default-src 'self'; script-src 'self' 'unsafe-inline' 'unsafe-eval' https://unpkg.com https://cdn.redoc.ly; style-src 'self' 'unsafe-inline' https://unpkg.com; img-src 'self' data: blob:; connect-src 'self'; font-src 'self' data:; frame-ancestors 'self'",
            );
            if let Ok(val) = HeaderValue::from_str(&csp) {
                headers.insert(actix_web::http::header::CONTENT_SECURITY_POLICY, val);
            }

            // X-Content-Type-Options
            headers.insert(
                actix_web::http::header::X_CONTENT_TYPE_OPTIONS,
                HeaderValue::from_static("nosniff"),
            );

            // X-Frame-Options
            headers.insert(
                actix_web::http::header::X_FRAME_OPTIONS,
                HeaderValue::from_static("SAMEORIGIN"),
            );

            // Strict-Transport-Security
            headers.insert(
                actix_web::http::header::STRICT_TRANSPORT_SECURITY,
                HeaderValue::from_static("max-age=31536000; includeSubDomains"),
            );

            // Referrer-Policy
            headers.insert(
                actix_web::http::header::REFERRER_POLICY,
                HeaderValue::from_static("strict-origin-when-cross-origin"),
            );

            // X-Powered-By
            headers.insert(
                actix_web::http::header::HeaderName::from_static("x-powered-by"),
                HeaderValue::from_static("Nexus"),
            );

            Ok(res)
        })
    }
}
