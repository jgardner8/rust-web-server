use std::collections::BTreeMap;
use std::fmt::Display;
use std::io::{self, prelude::Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use crate::web_server::request::Resource;
use crate::web_server::request_parser::ParseResult;
use crate::web_server::{Body, Request, RequestMethod, Response, request_parser};
use crate::{
    arc::Arc,
    thread_pool::ThreadPool,
    web_server::{ErrorRoute, Route, request_handler::RequestHandler},
};

const THREADS: usize = 20;
const READ_TIMEOUT: Duration = Duration::new(3, 0);
const WRITE_TIMEOUT: Duration = Duration::new(5, 0);
const SHUTDOWN_POLL_INTERVAL: Duration = Duration::from_millis(50);

pub fn bind_and_listen<A: ToSocketAddrs + Display>(
    address: A,
    routes: Box<[Route]>,
    error_routes: Box<[ErrorRoute]>,
) {
    let thread_pool = ThreadPool::new(THREADS);
    let listener = create_tcp_listener(address);
    let shutdown_requested = create_shutdown_signal_handler();
    let request_handler = Arc::new(RequestHandler::new(routes, error_routes));

    loop {
        if shutdown_requested.load(Ordering::SeqCst) {
            break;
        }

        match listener.accept() {
            Ok((tcp_stream, _addr)) => {
                tcp_stream
                    .set_nonblocking(false)
                    .expect("Fatal: Failed to set TcpStream to blocking mode");

                let request_handler = Arc::clone(&request_handler);

                thread_pool.execute(move || {
                    handle_connection(tcp_stream, request_handler);
                });
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                thread::sleep(SHUTDOWN_POLL_INTERVAL);
            }
            Err(e) => eprintln!("Client Error: Connection failed: {:?}", e),
        }
    }
}

fn create_tcp_listener<A: ToSocketAddrs + Display>(address: A) -> TcpListener {
    let listener = TcpListener::bind(&address).expect("Fatal: Failed to bind address");

    listener
        .set_nonblocking(true)
        .expect("Fatal: Failed to set TcpListener to non-blocking mode");

    println!("Listening on {}", &address);

    listener
}

fn create_shutdown_signal_handler() -> Arc<AtomicBool> {
    let shutdown_request_writer = Arc::new(AtomicBool::new(false));
    let shutdown_request_reader = Arc::clone(&shutdown_request_writer);

    ctrlc::set_handler(move || {
        println!("\nShutdown signal received, stopping...");
        shutdown_request_writer.store(true, Ordering::SeqCst);
    })
    .expect("Fatal: Failed to register signal handler");

    shutdown_request_reader
}

fn handle_connection(mut tcp_stream: TcpStream, request_handler: Arc<RequestHandler>) {
    tcp_stream
        .set_read_timeout(Some(READ_TIMEOUT))
        .expect("set_read_timeout system call failed");
    tcp_stream
        .set_write_timeout(Some(WRITE_TIMEOUT))
        .expect("set_write_timeout system call failed");

    let request_parse_result = request_parser::parse_stream(&tcp_stream);

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
