use std::str::FromStr;

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

    pub fn request_line_to_response(&self, request_line: &str) -> Response {
        let elems = request_line.split(" ").collect::<Vec<&str>>();

        match elems.as_slice() {
            [method, path, "HTTP/1.1"] if RequestMethod::from_str(method).is_ok() => {
                let method = RequestMethod::from_str(method).unwrap();

                let path_no_query_params = path.split("?").next().unwrap(); // unwrap is safe - split always returns at least one value
                let matched_pattern = self
                    .request_patterns
                    .iter()
                    .find(|pattern| pattern.matches(method, path_no_query_params));

                let resource = Resource::owned(String::from(*path));
                match matched_pattern {
                    Some(pattern) => {
                        pattern
                            .to_response(method, &resource)
                            .unwrap_or_else(|error_status_line| {
                                self.error_response(error_status_line, method, &resource)
                            })
                    }
                    None => self.error_response(StatusLine::new(404), method, &resource),
                }
            }
            _ => self.error_response(
                StatusLine::new(400),
                RequestMethod::Unknown,
                &Resource::borrowed(""),
            ),
        }
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
                .unwrap_or_else(|error_status_line| Response::from(error_status_line)),
            None => Response::from(status_line),
        }
    }
}
