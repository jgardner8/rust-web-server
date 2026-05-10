use http_server::web_server::{
    ErrorRoute, Request, RequestMethod::Get, Route, Response, StatusCode, WebServer,
};

fn route_query_params(request: &Request) -> Result<Response, StatusCode> {
    let elems = request.resource.path.split("?").collect::<Vec<&str>>();

    let body = if elems.len() == 1 {
        String::from("Dynamic page - call me with some query parameters!")
    } else {
        format!("Called {} with vars \"{}\"", elems[0], elems[1])
    };

    Ok(Response::new(StatusCode::Ok, body))
}

fn route_get_user(request: &Request) -> Result<Response, StatusCode> {
    Ok(Response::new(
        StatusCode::Ok,
        String::from(request.resource.path.clone()),
    ))
}

fn main() {
    WebServer::bind_and_listen_forever(
        "127.0.0.1:8080",
        Box::new([
            Route::file(Get, "/", "html/index.html"),
            Route::file(Get, "/index.html", "html/index.html"),
            Route::file(Get, "/other.html", "html/other.html"),
            Route::func(Get, "/query_params", route_query_params),
            Route::func(Get, "/user/{}", route_get_user),
        ]),
        Box::new([ErrorRoute::file(StatusCode::NotFound, "html/404_not_found.html")]),
    );
}
