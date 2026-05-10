use std::{
    collections::BTreeMap,
    io::{
        self, BufReader,
        prelude::{BufRead, Read},
    },
    str::FromStr,
};

use crate::web_server::{Request, RequestMethod, Resource, StatusLine};

pub struct RequestParser;

pub enum ParseResult {
    StreamError(io::Error),
    FailedOnRequestLine(StatusLine),
    FailedOnHeaders(StatusLine, RequestMethod, Resource),
    FailedOnBody(
        StatusLine,
        RequestMethod,
        Resource,
        BTreeMap<String, String>,
    ),
    Success(Request),
}

const ASSUMED_REQUEST_SIZE: usize = 128; // around basic GET request size from testing
const MAX_REQUEST_LINE_SIZE: u16 = 2 * 1024; // https://stackoverflow.com/questions/417142/what-is-the-maximum-length-of-a-url-in-different-browsers
const MAX_HEADERS_SIZE: u16 = 8 * 1024; // https://stackoverflow.com/questions/686217/maximum-on-http-header-values
const MAX_BODY_SIZE: usize = 1024 * 1024; // usually higher, but works for testing https://stackoverflow.com/questions/2880722/can-http-post-be-limitless

impl RequestParser {
    fn parse_request_line(request_line: &str) -> Result<(RequestMethod, Resource), StatusLine> {
        let elems = request_line.split(" ").collect::<Vec<&str>>();

        if elems.len() != 3 {
            return Err(StatusLine::new(400));
        }

        if elems[2] != "HTTP/1.1" {
            return Err(StatusLine::new(505));
        }

        let resource = Resource::owned(String::from(elems[1]));

        let method = RequestMethod::from_str(elems[0]).map_err(|()| StatusLine::new(501))?;

        Ok((method, resource))
    }

    fn parse_header(header_line: &str) -> Result<Option<(String, String)>, StatusLine> {
        if header_line.len() <= 2 {
            // Matches "\r\n" and "", while being too short for a valid header definition (a:b). Must be at end of headers
            Ok(None)
        } else {
            match header_line.split_once(":") {
                Some((k, v)) => Ok(Some((String::from(k.trim()), String::from(v.trim())))),
                None => Err(StatusLine::new(400)),
            }
        }
    }

    pub fn parse_stream<'a, T>(stream: &'a T) -> ParseResult
    where
        &'a T: io::Read,
    {
        let mut reader = BufReader::new(stream);
        let buf = &mut String::with_capacity(ASSUMED_REQUEST_SIZE);

        // Read request line
        match reader
            .by_ref()
            .take(MAX_REQUEST_LINE_SIZE.into())
            .read_line(buf)
        {
            Ok(bytes_read) if bytes_read < MAX_REQUEST_LINE_SIZE.into() => bytes_read,
            Ok(_) => return ParseResult::FailedOnRequestLine(StatusLine::new(414)),
            Err(e) => return ParseResult::StreamError(e),
        };

        let (method, resource) = match Self::parse_request_line(buf.trim_end()) {
            Ok((method, resource)) => (method, resource),
            Err(status_line) => return ParseResult::FailedOnRequestLine(status_line),
        };

        // Read headers
        buf.clear();
        let mut total_bytes_read: usize = 0;
        let mut headers = BTreeMap::new();
        loop {
            total_bytes_read += match reader.by_ref().take(MAX_HEADERS_SIZE.into()).read_line(buf) {
                Ok(bytes_read) => bytes_read,
                Err(e) => return ParseResult::StreamError(e),
            };
            if total_bytes_read > MAX_HEADERS_SIZE.into() {
                return ParseResult::FailedOnHeaders(StatusLine::new(431), method, resource);
            }

            match Self::parse_header(buf) {
                Ok(None) => break,
                Ok(Some((key, value))) => headers.insert(key, value),
                Err(status_line) => {
                    return ParseResult::FailedOnHeaders(status_line, method, resource);
                }
            };

            buf.clear();
        }

        // Read body
        buf.clear();
        let body_size_bytes = match headers.get("Content-Length") {
            Some(bytes_str) => match bytes_str.parse::<usize>() {
                Ok(bytes) if bytes < MAX_BODY_SIZE => bytes,
                Ok(_) => {
                    return ParseResult::FailedOnBody(
                        StatusLine::new(413),
                        method,
                        resource,
                        headers,
                    );
                }
                Err(_) => {
                    return ParseResult::FailedOnBody(
                        StatusLine::new(400),
                        method,
                        resource,
                        headers,
                    );
                }
            },
            None if reader.buffer().is_empty() => 0,
            None => {
                return ParseResult::FailedOnBody(StatusLine::new(411), method, resource, headers);
            }
        };
        match reader
            .by_ref()
            .take(body_size_bytes.try_into().unwrap())
            .read_line(buf)
        {
            Ok(_) => (),
            Err(e) => return ParseResult::StreamError(e),
        }
        let body = buf.clone();

        ParseResult::Success(Request::new(method, resource, headers, body))
    }
}
