use std::{
    collections::BTreeMap,
    convert,
    io::{
        self, BufReader,
        prelude::{BufRead, Read},
    },
    net::TcpStream,
    str::FromStr,
};

use crate::web_server::{
    ErrorPage, Request, RequestMethod, RequestPattern, Resource, Response, StatusLine,
};

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
                &Request::new(
                    RequestMethod::Unknown,
                    Resource::invalid(),
                    BTreeMap::new(),
                    String::new(),
                ),
            ));
        }

        if elems[2] != "HTTP/1.1" {
            return Err(self.error_response(
                StatusLine::new(505),
                &Request::new(
                    RequestMethod::Unknown,
                    Resource::invalid(),
                    BTreeMap::new(),
                    String::new(),
                ),
            ));
        }

        let path = String::from(elems[1]);

        let method = RequestMethod::from_str(elems[0]).map_err(|()| {
            self.error_response(
                StatusLine::new(501),
                &Request::new(
                    RequestMethod::Unknown,
                    Resource::owned(path.clone()),
                    BTreeMap::new(),
                    String::new(),
                ),
            )
        })?;

        Ok((method, Resource::owned(path)))
    }

    fn find_request_pattern(&self, request: &Request) -> Result<&RequestPattern, Response> {
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

    fn request_to_response(&self, request: &Request) -> Response {
        let maybe_request_pattern = self.find_request_pattern(request);
        let maybe_response = maybe_request_pattern
            .and_then(|request_pattern| self.request_pattern_to_response(request_pattern, request));
        maybe_response.unwrap_or_else(convert::identity) // both error and success Responses are valid HTTP Responses, collapse both sides of Result<>
    }

    pub fn request_stream_to_response(&self, request_stream: &TcpStream) -> io::Result<Response> {
        const ASSUMED_REQUEST_SIZE: usize = 128; // around basic GET request size from testing
        const MAX_REQUEST_LINE_SIZE: u16 = 2 * 1024; // https://stackoverflow.com/questions/417142/what-is-the-maximum-length-of-a-url-in-different-browsers
        const MAX_HEADERS_SIZE: u16 = 8 * 1024; // https://stackoverflow.com/questions/686217/maximum-on-http-header-values
        const MAX_BODY_SIZE: usize = 1024 * 1024; // usually higher, but works for testing https://stackoverflow.com/questions/2880722/can-http-post-be-limitless

        let mut reader = BufReader::new(request_stream);

        // Read request line
        let buf = &mut String::with_capacity(ASSUMED_REQUEST_SIZE);
        let bytes_read = reader
            .by_ref()
            .take(MAX_REQUEST_LINE_SIZE.into())
            .read_line(buf)?;
        if bytes_read >= MAX_REQUEST_LINE_SIZE.into() {
            return Ok(self.error_response(
                StatusLine::new(414),
                &Request::new(
                    RequestMethod::Unknown,
                    Resource::invalid(),
                    BTreeMap::new(),
                    String::new(),
                ),
            ));
        }
        let (method, resource) = match self.parse_request_line(buf.trim_end()) {
            Ok((method, resource)) => (method, resource),
            Err(response) => return Ok(response),
        };

        // Read headers
        buf.clear();
        let mut bytes_read: usize = 0;
        let mut headers = BTreeMap::new();
        loop {
            bytes_read += reader
                .by_ref()
                .take(MAX_HEADERS_SIZE.into())
                .read_line(buf)?;
            if bytes_read >= MAX_HEADERS_SIZE.into() {
                return Ok(self.error_response(
                    StatusLine::new(431),
                    &Request::new(method, resource, BTreeMap::new(), String::new()),
                ));
            }
            if buf.len() <= 2 {
                // matches "\r\n" and "", while being too short for a valid header definition (a:b)
                break;
            }

            match buf.split_once(":") {
                Some((k, v)) => headers.insert(String::from(k.trim()), String::from(v.trim())),
                None => {
                    return Ok(self.error_response(
                        StatusLine::new(400),
                        &Request::new(method, resource, BTreeMap::new(), String::new()),
                    ));
                }
            };

            buf.clear();
        }

        // Read body
        buf.clear();
        let body_size_bytes = match headers.get("Content-Length") {
            Some(bytes_str) => match bytes_str.parse::<usize>() {
                Ok(bytes) if bytes <= MAX_BODY_SIZE => bytes,
                Ok(_) => {
                    return Ok(self.error_response(
                        StatusLine::new(413),
                        &Request::new(method, resource, headers, String::new()),
                    ));
                }
                Err(_) => {
                    return Ok(self.error_response(
                        StatusLine::new(400),
                        &Request::new(method, resource, headers, String::new()),
                    ));
                }
            },
            None if reader.buffer().is_empty() => 0,
            None => {
                return Ok(self.error_response(
                    StatusLine::new(411),
                    &Request::new(method, resource, headers, String::new()),
                ));
            }
        };
        reader
            .by_ref()
            .take(body_size_bytes.try_into().unwrap())
            .read_line(buf)?;
        let body = buf.clone();

        // Build request
        let request = Request::new(method, resource, headers, body);
        println!("request {:?}", request);

        // Request to response
        let response = self.request_to_response(&request);

        println!(
            "{:?} {:?} -> {}",
            request.method,
            request.resource,
            response.status_line.encode_http_str()
        );

        Ok(response)
    }

    fn error_response(&self, status_line: StatusLine, request: &Request) -> Response {
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
