use std::{collections::BTreeMap, convert, io::{self, BufReader, prelude::{BufRead, Read}}, net::TcpStream, str::FromStr};

use crate::web_server::{ErrorPage, RequestMethod, RequestPattern, Resource, Response, StatusLine, Request};

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
                &Resource::invalid(),
            ));
        }

        if elems[2] != "HTTP/1.1" {
            return Err(self.error_response(
                StatusLine::new(505),
                RequestMethod::Unknown,
                &Resource::invalid(),
            ));
        }

        let resource = Resource::owned(String::from(elems[1]));

        let method = RequestMethod::from_str(elems[0]).map_err(|()| {
            self.error_response(StatusLine::new(501), RequestMethod::Unknown, &resource)
        })?;

        Ok((method, resource))
    }

    fn find_request_pattern(
        &self,
        method: RequestMethod,
        resource: &Resource,
    ) -> Result<&RequestPattern, Response> {
        let path_no_query_params = resource.path.split("?").next().unwrap(); // unwrap is safe - split always returns at least one value

        let matched_patterns_by_path = self
            .request_patterns
            .iter()
            .filter(|pattern| pattern.matches_path(path_no_query_params))
            .collect::<Vec<_>>();

        if matched_patterns_by_path.is_empty() {
            Err(self.error_response(StatusLine::new(404), method, resource))
        } else {
            matched_patterns_by_path
                .iter()
                .find(|pattern| pattern.matches(method, path_no_query_params))
                .ok_or(self.error_response(StatusLine::new(405), method, resource))
                .copied()
        }
    }

    fn request_pattern_to_response(
        &self,
        request_pattern: &RequestPattern,
        method: RequestMethod,
        resource: &Resource,
    ) -> Result<Response, Response> {
        request_pattern
            .to_response(method, resource)
            .map_err(|status_line| self.error_response(status_line, method, resource))
    }

    fn request_line_to_response_result(&self, request_line: &str) -> Result<Response, Response> {
        let (method, resource) = self.parse_request_line(request_line)?;
        let request_pattern = self.find_request_pattern(method, &resource)?;
        self.request_pattern_to_response(request_pattern, method, &resource)
    }

    fn request_line_to_response(&self, request_line: &str) -> Response {
        // Successful and unsuccessful responses are both valid HTTP responses - flatten Result type
        self.request_line_to_response_result(request_line)
            .unwrap_or_else(convert::identity)
    }

    pub fn request_stream_to_response(&self, request_stream: &TcpStream) -> io::Result<Response> {
        const ASSUMED_REQUEST_SIZE: usize = 128; // around basic GET request size from testing
        const MAX_REQUEST_LINE_SIZE: u16 = 2 * 1024; // https://stackoverflow.com/questions/417142/what-is-the-maximum-length-of-a-url-in-different-browsers
        const MAX_HEADERS_SIZE: u16 = 8 * 1024; // https://stackoverflow.com/questions/686217/maximum-on-http-header-values
        const MAX_BODY_SIZE: usize = 1 * 1024 * 1024; // usually higher, but works for testing https://stackoverflow.com/questions/2880722/can-http-post-be-limitless

        let mut reader = BufReader::new(request_stream);

        let buf = &mut String::with_capacity(ASSUMED_REQUEST_SIZE);
        let bytes_read = reader.by_ref().take(MAX_REQUEST_LINE_SIZE.into()).read_line(buf)?;
        if bytes_read >= MAX_REQUEST_LINE_SIZE.into() {
            return Ok(self.error_response(StatusLine::new(414), RequestMethod::Unknown, &Resource::invalid()))
        }

        let request_line = String::from(buf.trim_end());
        let response = self.request_line_to_response(&request_line);

        println!(
            "{} -> {}",
            request_line,
            response.status_line.encode_http_str()
        );

        buf.clear();
        let mut bytes_read: usize = 0;
        let mut headers = BTreeMap::new();
        loop {
            bytes_read += reader.by_ref().take(MAX_HEADERS_SIZE.into()).read_line(buf)?;
            if bytes_read >= MAX_HEADERS_SIZE.into() {
                return Ok(self.error_response(StatusLine::new(431), RequestMethod::Unknown, &Resource::invalid()))
            }
            if buf.len() <= 2 { // matches "\r\n" and "", while being too short for a valid header definition (a:b)
                break
            }

            match buf.split_once(":") {
                Some((k, v)) => headers.insert(String::from(k.trim()), String::from(v.trim())),
                None => return Ok(self.error_response(StatusLine::new(400), RequestMethod::Unknown, &Resource::invalid()))
            };
            
            buf.clear();
        }

        buf.clear();
        let body_size_bytes = match headers.get("Content-Length") {
            Some(bytes_str) => match bytes_str.parse::<usize>() {
                Ok(bytes) if bytes <= MAX_BODY_SIZE => bytes,
                Ok(_) => return Ok(self.error_response(StatusLine::new(413), RequestMethod::Unknown, &Resource::invalid())),
                Err(_) => return Ok(self.error_response(StatusLine::new(400), RequestMethod::Unknown, &Resource::invalid()))
            }
            None if reader.buffer().is_empty() => 0,
            None => return Ok(self.error_response(StatusLine::new(411), RequestMethod::Unknown, &Resource::invalid()))
        };
    
        reader.by_ref().take(body_size_bytes.try_into().unwrap()).read_line(buf)?;

        let request = Request::new(RequestMethod::Unknown, Resource::invalid(), headers, buf.clone());

        println!("request {:?}", request);

        Ok(response)
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
