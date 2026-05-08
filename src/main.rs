use web_server::WebServer;

fn main() {
    WebServer::bind_and_listen_forever("127.0.0.1:8080");
}
