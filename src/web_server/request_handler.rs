use std::convert;
use std::net::IpAddr;

use crate::vec::Vec;
use crate::web_server::rate_limiter::RateLimiter;
use crate::web_server::{ErrorRoute, Request, Response, Route, StatusCode};

pub struct RequestHandler {
    routes: Box<[Route]>,
    error_routes: Box<[ErrorRoute]>,
    rate_limiter: RateLimiter<IpAddr>
}

impl RequestHandler {
    pub fn new(routes: Box<[Route]>, error_routes: Box<[ErrorRoute]>, rate_limiter: RateLimiter<IpAddr>) -> Self {
        RequestHandler {
            routes,
            error_routes,
            rate_limiter
        }
    }

    fn find_matching_route(&self, request: &Request) -> Result<&Route, Response> {
        let matched_routes_by_path = self
            .routes
            .iter()
            .filter(|route| route.matches_path(&request.resource.path))
            .collect::<Vec<_>>();

        if matched_routes_by_path.is_empty() {
            Err(self.error_response(StatusCode::NotFound, request))
        } else {
            matched_routes_by_path
                .into_iter()
                .find(|route| route.matches_method(request.method))
                .ok_or(self.error_response(StatusCode::MethodNotAllowed, request))
        }
    }

    fn route_to_response(&self, route: &Route, request: &Request) -> Result<Response, Response> {
        route
            .to_response(request)
            .map_err(|status_code| self.error_response(status_code, request))
    }

    fn error_response(&self, status_code: StatusCode, request: &Request) -> Response {
        match self
            .error_routes
            .iter()
            .find(|error_route| error_route.matches(status_code))
        {
            Some(error_route) => error_route
                .to_response(request)
                .unwrap_or_else(Response::from),
            None => Response::from(status_code),
        }
    }

    pub fn handle_request(&self, request: &Request) -> Response {
        if self.rate_limiter.client_over_rate_limit(request.addr) {
            return self.error_response(StatusCode::TooManyRequests, request);
        }

        self.find_matching_route(request)
            .and_then(|route| self.route_to_response(route, request))
            .unwrap_or_else(convert::identity) // both error and success Responses are valid HTTP Responses, collapse both sides of Result<>
    }

    pub fn handle_error(&self, status_code: StatusCode, request: &Request) -> Response {
        if self.rate_limiter.client_over_rate_limit(request.addr) {
            return self.error_response(StatusCode::TooManyRequests, request);
        }
        
        self.error_response(status_code, request)
    }
}
