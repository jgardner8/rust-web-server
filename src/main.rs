use http_server::web_server::{
    ErrorPage,
    RequestMethod::{self, Get},
    RequestPattern, Resource, Response, StatusLine, WebServer,
};

fn route_dynamic(_method: RequestMethod, resource: &Resource) -> Result<Response, StatusLine> {
    let elems = resource.path.split("?").collect::<Vec<&str>>();

    let body = if elems.len() == 1 {
        String::from("Dynamic page - call me with some query variables!")
    } else {
        format!("Called {} with vars \"{}\"", elems[0], elems[1])
    };

    Ok(Response::new(StatusLine::new(200), body))
}

fn main() {
    WebServer::bind_and_listen_forever(
        "127.0.0.1:8080",
        Box::new([
            RequestPattern::file(Get, "/", "html/index.html"),
            RequestPattern::file(Get, "/index.html", "html/index.html"),
            RequestPattern::file(Get, "/other.html", "html/other.html"),
            RequestPattern::function(Get, "/dynamic", route_dynamic),
        ]),
        Box::new([ErrorPage::file(404, "html/404.html")]),
    );
}
