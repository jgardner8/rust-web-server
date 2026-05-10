use std::{convert, str::FromStr};

use crate::web_server::{ErrorPage, RequestMethod, RequestPattern, Resource, Response, StatusLine};

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

    fn parse_request_line(
        &self,
        request_line: &str,
    ) -> Result<(RequestMethod, Resource), Response> {
        let elems = request_line.split(" ").collect::<Vec<&str>>();

        if elems.len() != 3 {
            return Err(self.error_response(
                StatusLine::new(400),
                RequestMethod::Unknown,
                &Resource::borrowed(""),
            ));
        }

        if elems[2] != "HTTP/1.1" {
            return Err(self.error_response(
                StatusLine::new(505),
                RequestMethod::Unknown,
                &Resource::borrowed(""),
            ));
        }

        let resource = Resource::owned(String::from(elems[1]));

        let method = RequestMethod::from_str(elems[0]).map_err(|()| {
            self.error_response(
                StatusLine::new(501),
                RequestMethod::Unknown,
                &resource,
            )
        })?;

        Ok((method, resource))
    }

    fn find_request_pattern(
        &self,
        method: RequestMethod,
        resource: &Resource,
    ) -> Result<&RequestPattern, Response> {
        let path_no_query_params = resource.path.split("?").next().unwrap(); // unwrap is safe - split always returns at least one value
        let matched_pattern = self
            .request_patterns
            .iter()
            .find(|pattern| pattern.matches(method, path_no_query_params));

        matched_pattern.ok_or(self.error_response(StatusLine::new(404), method, &resource))
    }

    fn request_pattern_to_response(
        &self,
        request_pattern: &RequestPattern,
        method: RequestMethod,
        resource: &Resource,
    ) -> Result<Response, Response> {
        request_pattern
            .to_response(method, &resource)
            .map_err(|status_line| self.error_response(status_line, method, &resource))
    }

    fn request_line_to_response_result(&self, request_line: &str) -> Result<Response, Response> {
        let (method, resource) = self.parse_request_line(request_line)?;
        let request_pattern = self.find_request_pattern(method, &resource)?;
        self.request_pattern_to_response(request_pattern, method, &resource)
    }

    pub fn request_line_to_response(&self, request_line: &str) -> Response {
        self.request_line_to_response_result(request_line).unwrap_or_else(convert::identity)
    }

    fn error_response(
        &self,
        status_line: StatusLine,
        method: RequestMethod,
        resource: &Resource,
    ) -> Response {
        match self
            .error_pages
            .iter()
            .find(|error_page| error_page.matches(status_line.code))
        {
            Some(error_page) => error_page
                .to_response(method, resource)
                .unwrap_or_else(Response::from),
            None => Response::from(status_line),
        }
    }
}
