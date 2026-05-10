use http_server::web_server::{
    ErrorRoute, Request, RequestMethod::Get, Response, Route, StatusCode, WebServer,
};

fn route_query_params(request: &Request) -> Result<Response, StatusCode> {
    let elems = request.resource.path.split("?").collect::<Vec<&str>>();

    let body = if elems.len() == 1 {
        "Dynamic page - call me with some query parameters!".to_string()
    } else {
        format!("Called {} with vars \"{}\"", elems[0], elems[1])
    };

    Ok(Response::ok(body))
}

fn route_get_user(request: &Request) -> Result<Response, StatusCode> {
    match request.headers.get("user-cookie") {
        Some(cookie) if cookie == "test" => Ok(Response::ok("Welcome user!".to_string())),
        Some(_) => Err(StatusCode::Forbidden),
        None => Err(StatusCode::Unauthorized),
    }
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
        Box::new([ErrorRoute::file(
            StatusCode::NotFound,
            "html/404_not_found.html",
        )]),
    );
}
