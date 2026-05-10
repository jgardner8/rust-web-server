use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::io::{prelude::Write};

use crate::{
    arc::Arc,
    thread_pool::ThreadPool,
    web_server::{ErrorPage, RequestPattern, request_handler::RequestHandler},
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

        let request_handler = Arc::new(RequestHandler::new(request_patterns, error_pages));

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
    match request_handler.request_stream_to_response(&tcp_stream) {
        Ok(response) => {
            tcp_stream
                .write_all(response.encode_http_str().as_bytes())
                .unwrap_or_else(|e| {
                    eprintln!("Error: Failed to write response: {:?}", e);
                });
        }
        Err(e) => {
            println!("Client Error: Failed to receive full request: {:?}", e);
        }
    }
}
