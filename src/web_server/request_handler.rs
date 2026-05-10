use std::convert;

use crate::web_server::{ErrorPage, Request, RequestPattern, Response, StatusLine};

pub struct RequestHandler {
    request_patterns: Box<[RequestPattern]>,
    error_pages: Box<[ErrorPage]>,
}

impl RequestHandler {
    pub fn new(request_patterns: Box<[RequestPattern]>, error_pages: Box<[ErrorPage]>) -> Self {
        RequestHandler {
            request_patterns,
            error_pages,
        }
    }

    fn find_matching_request_pattern(
        &self,
        request: &Request,
    ) -> Result<&RequestPattern, Response> {
        let path_no_query_params = request.resource.path.split("?").next().unwrap(); // unwrap is safe - split always returns at least one value

        let matched_patterns_by_path = self
            .request_patterns
            .iter()
            .filter(|pattern| pattern.matches_path(path_no_query_params))
            .collect::<Vec<_>>();

        if matched_patterns_by_path.is_empty() {
            Err(self.error_response(StatusLine::new(404), request))
        } else {
            matched_patterns_by_path
                .iter()
                .find(|pattern| pattern.matches(request.method, path_no_query_params))
                .ok_or(self.error_response(StatusLine::new(405), request))
                .copied()
        }
    }

    fn request_pattern_to_response(
        &self,
        request_pattern: &RequestPattern,
        request: &Request,
    ) -> Result<Response, Response> {
        request_pattern
            .to_response(request)
            .map_err(|status_line| self.error_response(status_line, request))
    }

    pub fn handle_request(&self, request: &Request) -> Response {
        let maybe_request_pattern = self.find_matching_request_pattern(request);
        let maybe_response =
            maybe_request_pattern.and_then(|pat| self.request_pattern_to_response(pat, request));
        maybe_response.unwrap_or_else(convert::identity) // both error and success Responses are valid HTTP Responses, collapse both sides of Result<>
    }

    pub fn error_response(&self, status_line: StatusLine, request: &Request) -> Response {
        match self
            .error_pages
            .iter()
            .find(|error_page| error_page.matches(status_line.code))
        {
            Some(error_page) => error_page
                .to_response(request)
                .unwrap_or_else(Response::from),
            None => Response::from(status_line),
        }
    }
}
