use std::{
    collections::BTreeMap,
    io::{
        self, BufReader,
        prelude::{BufRead, Read},
    },
    str::FromStr,
    string::FromUtf8Error,
};

use crate::web_server::{Body, Json, Request, RequestMethod, StatusCode, request::Parameters};
use crate::{vec::Vec, web_server::request::Resource};

pub enum ParseResult {
    StreamError(io::Error),
    FailedOnRequestLine(StatusCode),
    FailedOnHeaders(StatusCode, RequestMethod, Resource),
    FailedOnBody(StatusCode, RequestMethod, Resource, Parameters),
    Success(Request),
}

const ASSUMED_REQUEST_SIZE: usize = 128; // around basic GET request size from testing
const MAX_REQUEST_LINE_SIZE: u16 = 2 * 1024; // https://stackoverflow.com/questions/417142/what-is-the-maximum-length-of-a-url-in-different-browsers
const MAX_HEADERS_SIZE: u16 = 8 * 1024; // https://stackoverflow.com/questions/686217/maximum-on-http-header-values
const MAX_BODY_SIZE: usize = 1024 * 1024; // usually higher, but works for testing https://stackoverflow.com/questions/2880722/can-http-post-be-limitless

fn parse_query_str(query_str: &str) -> Parameters {
    let mut map = BTreeMap::new();
    for kvp in query_str.split('&') {
        match kvp.split_once('=') {
            None => continue,
            Some((key, value)) => map.insert(String::from(key), String::from(value)),
        };
    }
    map
}

fn parse_resource(url: &str) -> Result<Resource, FromUtf8Error> {
    let (path, query_params) = match url.split_once('?') {
        None => (url, BTreeMap::new()),
        Some((path, query_str)) => (path, parse_query_str(query_str)),
    };

    Resource::url_decode(path, query_params)
}

fn parse_request_line(request_line: &str) -> Result<(RequestMethod, Resource), StatusCode> {
    let elems = request_line.split(" ").collect::<Vec<&str>>();

    if elems.len() != 3 {
        return Err(StatusCode::BadRequest);
    }

    if elems[2] != "HTTP/1.1" {
        return Err(StatusCode::HttpVersionNotSupported);
    }

    let resource = match parse_resource(elems[1]) {
        Ok(resource) => resource,
        Err(_) => return Err(StatusCode::UnsupportedMediaType),
    };

    let method = RequestMethod::from_str(elems[0]).map_err(|()| StatusCode::NotImplemented)?;

    Ok((method, resource))
}

fn parse_header(header_line: &str) -> Result<Option<(String, String)>, StatusCode> {
    if header_line.len() <= 2 {
        // Matches "\r\n" and "", while being too short for a valid header definition (a:b). Must be at end of headers
        Ok(None)
    } else {
        match header_line.split_once(':') {
            Some((k, v)) => Ok(Some((String::from(k.trim()), String::from(v.trim())))),
            None => Err(StatusCode::BadRequest),
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
        Ok(_) => return ParseResult::FailedOnRequestLine(StatusCode::URITooLong),
        Err(e) => return ParseResult::StreamError(e),
    };

    let (method, resource) = match parse_request_line(buf.trim_end()) {
        Ok((method, resource)) => (method, resource),
        Err(status_code) => return ParseResult::FailedOnRequestLine(status_code),
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
            return ParseResult::FailedOnHeaders(
                StatusCode::RequestHeaderFieldsTooLarge,
                method,
                resource,
            );
        }

        match parse_header(buf) {
            Ok(None) => break,
            Ok(Some((key, value))) => headers.insert(key, value),
            Err(status_code) => {
                return ParseResult::FailedOnHeaders(status_code, method, resource);
            }
        };

        buf.clear();
    }

    // Read body
    buf.clear();
    let body_size_bytes = match headers
        .get("Content-Length")
        .and_then(|v| v.parse::<usize>().ok())
    {
        Some(bytes) if bytes < MAX_BODY_SIZE => bytes,
        Some(_) => {
            return ParseResult::FailedOnBody(
                StatusCode::ContentTooLarge,
                method,
                resource,
                headers,
            );
        }
        None if reader.buffer().is_empty() => 0,
        None => {
            return ParseResult::FailedOnBody(
                StatusCode::LengthRequired,
                method,
                resource,
                headers,
            );
        }
    };
    match reader
        .by_ref()
        .take(body_size_bytes.try_into().unwrap())
        .read_to_string(buf)
    {
        Ok(_) => (),
        Err(e) => return ParseResult::StreamError(e),
    }

    let body = match headers.get("Content-Type").map(|s| s.as_str()) {
        Some("application/json") => match Json::parse(buf) {
            Ok(json) => Body::JsonData(json),
            Err(_) => {
                return ParseResult::FailedOnBody(
                    StatusCode::BadRequest,
                    method,
                    resource,
                    headers,
                );
            }
        },
        Some("application/x-www-form-urlencoded") => Body::FormData(parse_query_str(buf)),
        _ => Body::Text(buf.clone()),
    };

    ParseResult::Success(Request::new(method, resource, headers, body))
}

impl ParseResult {
    pub fn to_log(&self) -> String {
        match self {
            ParseResult::StreamError(e) => {
                format!("Client Error: Connection closed prematurely: {:?}\n", e)
            }
            ParseResult::FailedOnRequestLine(_) => {
                "Client Error: Cannot parse request line".to_string()
            }
            ParseResult::FailedOnHeaders(_, method, resource) => format!(
                "Client Error: Cannot parse headers - {:?} {}",
                method, resource.path
            ),
            ParseResult::FailedOnBody(_, method, resource, _) => format!(
                "Client Error: Cannot parse body - {:?} {}",
                method, resource.path
            ),
            ParseResult::Success(request) => request.to_log(),
        }
    }
}
