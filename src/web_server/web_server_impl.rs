use std::{
    io::{BufReader, Error, ErrorKind, prelude::*},
    net::{TcpListener, TcpStream, ToSocketAddrs},
};

use crate::{
    arc::Arc,
    thread_pool::ThreadPool,
    web_server::{
        RequestPattern,
        request_handler::{ErrorPage, RequestHandler},
    },
};

pub struct WebServer;

impl WebServer {
    pub fn bind_and_listen_forever<A: ToSocketAddrs>(
        address: A,
        request_patterns: Box<[RequestPattern]>,
        error_pages: Box<[ErrorPage]>,
    ) {
        let thread_pool = ThreadPool::new(4);
        let listener = TcpListener::bind(address).expect("Fatal: Failed to bind address");

        let request_matcher = Arc::new(RequestHandler::new(request_patterns, error_pages));

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let request_matcher = request_matcher.clone();
                    thread_pool.execute(move || {
                        handle_connection(stream, request_matcher);
                    });
                }
                Err(e) => {
                    println!("Connection error: {:?}", e);
                }
            }
        }
    }
}

fn handle_connection(mut stream: TcpStream, request_matcher: Arc<RequestHandler>) {
    match get_request_line(&stream) {
        Ok(request_line) => {
            let response = request_matcher.map_to_response(&request_line);

            println!("{} -> {}", request_line, response.status_code);

            stream
                .write_all(response.encode_http_str().as_bytes())
                .unwrap_or_else(|e| {
                    println!("Failed to write response: {:?}", e);
                });
        }
        Err(e) => {
            println!("Got no input from request: {:?}", e);
        }
    }
}

fn get_request_line(stream: &TcpStream) -> Result<String, Error> {
    let buf_reader = BufReader::new(stream);

    let request_line = buf_reader.lines().next();
    request_line.unwrap_or(Err(Error::from(ErrorKind::ConnectionAborted)))
}
