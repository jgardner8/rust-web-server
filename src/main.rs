use std::{
    fs,
    io::{BufReader, Error, ErrorKind, prelude::*},
    net::{TcpListener, TcpStream},
};

use web_server::thread_pool::ThreadPool;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").expect("Failed to bind address");
    let pool = ThreadPool::new(4);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                pool.execute(|| {
                    handle_connection(stream);
                });
            }
            Err(e) => {
                println!("Connection error: {:?}", e);
            }
        }
    }
}

fn get_request_line(stream: &TcpStream) -> Result<String, Error> {
    let buf_reader = BufReader::new(stream);

    let request_line = buf_reader.lines().next();
    request_line.unwrap_or(Err(Error::from(ErrorKind::ConnectionAborted)))
}

fn map_request_line_to_response(request_line: String) -> String { 
    let (status_line, filename) = if request_line == "GET / HTTP/1.1" {
        ("HTTP/1.1 200 OK", "./html/index.html")
    } else {
        ("HTTP/1.1 404 NOT FOUND", "./html/404.html")
    };

    match fs::read_to_string(filename) {
        Ok(response_body) => {
            let len = response_body.len();
            format!("{status_line}\nContent-Type: text/html; charset=utf-8\nContent-Length: {len}\n\n{response_body}")
        }
        Err(e) => {
            eprintln!("Error: Cannot read file {}, error: {:?}", filename, e);
            String::from("HTTP/1.1 404 NOT FOUND\n\nPage not found")       
        }
    }
}

fn handle_connection(mut stream: TcpStream) {
    match get_request_line(&stream) {
        Ok(request_line) => {
            let response = map_request_line_to_response(request_line);
            stream.write_all(response.as_bytes()).unwrap_or_else(|e| {
                println!("Failed to response to request: {:?}", e);
            });
        }
        Err(e) => {
            println!("Got no input from request: {:?}", e);
        }
    }
}
