use http_server::web_server::{
    ErrorPage,
    RequestMethod::{self, Get},
    RequestPattern, Resource, Response,
    ResponseType::{File, Function},
    WebServer,
};

fn route_dynamic(_method: RequestMethod, resource: Resource) -> Response {
    let elems = resource.path.split("?").collect::<Vec<&str>>();

    let body = if elems.len() == 1 {
        String::from("Dynamic page - call me with some query variables!")
    } else {
        format!("Called {} with vars \"{}\"", elems[0], elems[1])
    };

    Response::new(200, body)
}

fn main() {
    WebServer::bind_and_listen_forever(
        "127.0.0.1:8080",
        Box::new([
            RequestPattern::new(Get, "/", File("html/index.html")),
            RequestPattern::new(Get, "/index.html", File("html/index.html")),
            RequestPattern::new(Get, "/other.html", File("html/other.html")),
            RequestPattern::new(Get, "/dynamic", Function(Box::new(route_dynamic))),
        ]),
        Box::new([ErrorPage::new(404, File("html/404.html"))]),
    );
}
