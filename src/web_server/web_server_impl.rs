use std::collections::BTreeMap;
use std::io::prelude::Write;
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::time::Duration;

use crate::web_server::request_parser::ParseResult;
use crate::web_server::{Body, Request, RequestMethod, RequestParser, Resource, Response};
use crate::{
    arc::Arc,
    thread_pool::ThreadPool,
    web_server::{ErrorRoute, Route, request_handler::RequestHandler},
};

pub struct WebServer;

const READ_TIMEOUT: Duration = Duration::new(3, 0);
const WRITE_TIMEOUT: Duration = Duration::new(5, 0);

impl WebServer {
    pub fn bind_and_listen_forever<A: ToSocketAddrs>(
        address: A,
        routes: Box<[Route]>,
        error_routes: Box<[ErrorRoute]>,
    ) {
        let thread_pool = ThreadPool::new(4);
        let listener = TcpListener::bind(address).expect("Fatal: Failed to bind address");

        let request_handler = Arc::new(RequestHandler::new(routes, error_routes));

        for tcp_stream in listener.incoming() {
            match tcp_stream {
                Ok(tcp_stream) => {
                    let request_handler = request_handler.clone();
                    thread_pool.execute(move || {
                        handle_connection(tcp_stream, request_handler);
                    });
                }
                Err(e) => println!("Client Error: Connection failed: {:?}", e),
            }
        }
    }
}

fn handle_connection(mut tcp_stream: TcpStream, request_handler: Arc<RequestHandler>) {
    tcp_stream
        .set_read_timeout(Some(READ_TIMEOUT))
        .expect("set_read_timeout system call failed");
    tcp_stream
        .set_write_timeout(Some(WRITE_TIMEOUT))
        .expect("set_write_timeout system call failed");

    let request_parse_result = RequestParser::parse_stream(&tcp_stream);

    print!("{}", request_parse_result.to_log());

    if let Some(response) = handle_request_parse_result(request_parse_result, request_handler) {
        println!(" -> {}", response.to_log());

        tcp_stream
            .write_all(response.encode_http_str().as_bytes())
            .unwrap_or_else(|e| {
                eprintln!("Error: Failed to write response: {:?}", e);
            });
    }
}

fn handle_request_parse_result(
    request_parse_result: ParseResult,
    request_handler: Arc<RequestHandler>,
) -> Option<Response> {
    match request_parse_result {
        ParseResult::StreamError(_) => None,
        ParseResult::FailedOnRequestLine(status_code) => Some(request_handler.handle_error(
            status_code,
            &Request::new(
                RequestMethod::Unknown,
                Resource::invalid(),
                BTreeMap::new(),
                Body::Text(String::new()),
            ),
        )),
        ParseResult::FailedOnHeaders(status_code, method, resource) => {
            Some(request_handler.handle_error(
                status_code,
                &Request::new(method, resource, BTreeMap::new(), Body::Text(String::new())),
            ))
        }
        ParseResult::FailedOnBody(status_code, method, resource, headers) => {
            Some(request_handler.handle_error(
                status_code,
                &Request::new(method, resource, headers, Body::Text(String::new())),
            ))
        }
        ParseResult::Success(request) => Some(request_handler.handle_request(&request)),
    }
}
