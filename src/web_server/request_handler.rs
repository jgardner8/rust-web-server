use std::{fs, str::FromStr};

#[derive(PartialEq, Debug)]
pub enum RequestMethod {
    Get,
    Head,
    Post,
    Put,
    Delete,
    Patch,
    Unknown,
}

pub enum ResponseType {
    File(&'static str),
    Function(Box<RequestProcessorFn>),
}

type RequestProcessorFn = dyn Fn(RequestMethod, Resource) -> Response + Sync;

pub struct Resource<'a> {
    pub path: &'a str,
}

pub struct RequestPattern {
    method: RequestMethod,
    resource: Resource<'static>,
    response_type: ResponseType,
}

pub struct RequestHandler {
    request_patterns: Box<[RequestPattern]>,
    error_pages: Box<[ErrorPage]>,
}

pub struct Response {
    pub status_code: u16,
    body: String,
}

pub struct ErrorPage {
    status_code: u16,
    response_type: ResponseType,
}

impl FromStr for RequestMethod {
    type Err = ();
    fn from_str(input: &str) -> Result<RequestMethod, Self::Err> {
        match input {
            "GET" => Ok(RequestMethod::Get),
            "HEAD" => Ok(RequestMethod::Head),
            "POST" => Ok(RequestMethod::Post),
            "PUT" => Ok(RequestMethod::Put),
            "DELETE" => Ok(RequestMethod::Delete),
            "PATCH" => Ok(RequestMethod::Patch),
            _ => Err(()),
        }
    }
}

impl<'a> Resource<'a> {
    pub fn new(path: &'a str) -> Self {
        Resource { path }
    }
}

impl RequestPattern {
    pub fn new(method: RequestMethod, path: &'static str, response_type: ResponseType) -> Self {
        RequestPattern {
            method,
            resource: Resource { path },
            response_type,
        }
    }
}

impl RequestHandler {
    pub fn new(request_patterns: Box<[RequestPattern]>, error_pages: Box<[ErrorPage]>) -> Self {
        RequestHandler {
            request_patterns,
            error_pages,
        }
    }

    pub fn map_to_response(&self, request_line: &str) -> Response {
        let elems = request_line.split(" ").collect::<Vec<&str>>();

        match elems.as_slice() {
            [method, path, "HTTP/1.1"] if RequestMethod::from_str(method).is_ok() => {
                let method = RequestMethod::from_str(method).unwrap();

                let path_no_vars = path.split("?").next().unwrap(); // unwrap is safe - split always returns at least one value
                let matched_pattern = self
                    .request_patterns
                    .iter()
                    .find(|p| p.method == method && p.resource.path == path_no_vars);

                let resource = Resource::new(path);
                match matched_pattern {
                    Some(pattern) => {
                        self.response_from_type(&pattern.response_type, method, resource, false)
                    }
                    None => self.error_response(404, method, resource),
                }
            }
            _ => self.error_response(400, RequestMethod::Unknown, Resource::new("")),
        }
    }

    fn response_from_type(
        &self,
        response_type: &ResponseType,
        method: RequestMethod,
        resource: Resource,
        is_error_response: bool,
    ) -> Response {
        match response_type {
            ResponseType::File(path) => {
                self.file_response(method, resource, path, is_error_response)
            }
            ResponseType::Function(f) => f(method, resource),
        }
    }

    fn file_response(
        &self,
        method: RequestMethod,
        resource: Resource,
        path: &str,
        is_error_response: bool,
    ) -> Response {
        match fs::read_to_string(path) {
            Ok(response_body) => Response::new(200, response_body),
            Err(e) => {
                eprintln!("Error: Cannot read file {}, error: {:?}", path, e);

                // Exits infinite failure loop when failing to load error page
                if is_error_response {
                    Response::new(500, String::from("Internal Server Error"))
                } else {
                    self.error_response(404, method, resource)
                }
            }
        }
    }

    fn error_response(
        &self,
        status_code: u16,
        method: RequestMethod,
        resource: Resource,
    ) -> Response {
        match self
            .error_pages
            .iter()
            .find(|p| p.status_code == status_code)
        {
            Some(error_page) => {
                self.response_from_type(&error_page.response_type, method, resource, true)
            }
            None => Response::new(status_code, String::default()),
        }
    }
}

impl Response {
    pub fn new(status_code: u16, body: String) -> Self {
        Response { status_code, body }
    }

    pub fn encode_http_str(&self) -> String {
        format!(
            concat!(
                "HTTP/1.1 {}\n",
                "Content-Type: text/html; charset=utf-8\n",
                "Content-Length: {}\n",
                "\n",
                "{}"
            ),
            self.status_code,
            self.body.len(),
            self.body
        )
    }
}

impl ErrorPage {
    pub fn new(status_code: u16, response_type: ResponseType) -> Self {
        ErrorPage {
            status_code,
            response_type,
        }
    }
}
