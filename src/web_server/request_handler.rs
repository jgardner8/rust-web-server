use std::{borrow::Cow, fs, str::FromStr};

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum RequestMethod {
    Get,
    Head,
    Post,
    Put,
    Delete,
    Patch,
    Unknown,
}

pub struct Resource {
    pub path: Cow<'static, str>,
}

pub struct RequestPattern {
    method: RequestMethod,
    resource: Resource,
    response_type: ResponseType,
}

pub struct StatusLine {
    pub code: u16,
}

pub struct Response {
    pub status_line: StatusLine,
    body: String,
}

pub struct ErrorPage {
    status_code: u16,
    response_type: ResponseType,
}

pub struct RequestHandler {
    request_patterns: Box<[RequestPattern]>,
    error_pages: Box<[ErrorPage]>,
}

enum ResponseType {
    File(Cow<'static, str>),
    Function(Box<RequestProcessorFn>),
}

type RequestProcessorFn =
    dyn Fn(RequestMethod, &Resource) -> Result<Response, StatusLine> + Send + Sync;

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

impl ResponseType {
    fn new_file(path: &'static str) -> ResponseType {
        ResponseType::File(Cow::Borrowed(path))
    }

    fn new_file_dynamic(path: String) -> ResponseType {
        ResponseType::File(Cow::Owned(path))
    }

    fn new_function<F>(function: F) -> ResponseType
    where
        F: Fn(RequestMethod, &Resource) -> Result<Response, StatusLine> + Send + Sync + 'static,
    {
        ResponseType::Function(Box::new(function))
    }

    pub fn to_response(
        &self,
        method: RequestMethod,
        resource: &Resource,
    ) -> Result<Response, StatusLine> {
        match self {
            ResponseType::File(path) => self.file_response(path),
            ResponseType::Function(f) => f(method, resource),
        }
    }

    fn file_response(&self, path: &str) -> Result<Response, StatusLine> {
        match fs::read_to_string(path) {
            Ok(response_body) => Ok(Response::new(StatusLine::new(200), response_body)),
            Err(e) => {
                eprintln!("Error: Cannot read file {}, error: {:?}", path, e);
                Err(StatusLine::new(404))
            }
        }
    }
}

impl Resource {
    fn new(path: Cow<'static, str>) -> Self {
        Resource { path }
    }

    pub fn borrowed(path: &'static str) -> Self {
        Resource::new(Cow::Borrowed(path))
    }

    pub fn owned(path: String) -> Self {
        Resource::new(Cow::Owned(path))
    }
}

impl RequestPattern {
    fn new(method: RequestMethod, resource: Resource, response_type: ResponseType) -> Self {
        RequestPattern {
            method,
            resource,
            response_type,
        }
    }

    pub fn file(method: RequestMethod, path: &'static str, file_path: &'static str) -> Self {
        RequestPattern::new(
            method,
            Resource::borrowed(path),
            ResponseType::new_file(file_path),
        )
    }

    pub fn file_dynamic(method: RequestMethod, path: String, file_path: String) -> Self {
        RequestPattern::new(
            method,
            Resource::owned(path),
            ResponseType::new_file_dynamic(file_path),
        )
    }

    pub fn function<F>(method: RequestMethod, path: &'static str, function: F) -> Self
    where
        F: Fn(RequestMethod, &Resource) -> Result<Response, StatusLine> + Send + Sync + 'static,
    {
        RequestPattern::new(
            method,
            Resource::borrowed(path),
            ResponseType::new_function(function),
        )
    }

    pub fn function_dynamic<F>(method: RequestMethod, path: String, function: F) -> Self
    where
        F: Fn(RequestMethod, &Resource) -> Result<Response, StatusLine> + Send + Sync + 'static,
    {
        RequestPattern::new(
            method,
            Resource::owned(path),
            ResponseType::new_function(function),
        )
    }
}

impl RequestHandler {
    pub fn new(request_patterns: Box<[RequestPattern]>, error_pages: Box<[ErrorPage]>) -> Self {
        RequestHandler {
            request_patterns,
            error_pages,
        }
    }

    pub fn request_line_to_response(&self, request_line: &str) -> Response {
        let elems = request_line.split(" ").collect::<Vec<&str>>();

        match elems.as_slice() {
            [method, path, "HTTP/1.1"] if RequestMethod::from_str(method).is_ok() => {
                let method = RequestMethod::from_str(method).unwrap();

                let path_no_vars = path.split("?").next().unwrap(); // unwrap is safe - split always returns at least one value
                let matched_pattern = self
                    .request_patterns
                    .iter()
                    .find(|p| p.method == method && p.resource.path == path_no_vars);

                let resource = Resource::owned(String::from(*path));
                match matched_pattern {
                    Some(pattern) => pattern
                        .response_type
                        .to_response(method, &resource)
                        .unwrap_or_else(|error_status_line| {
                            self.error_response(error_status_line, method, &resource)
                        }),
                    None => self.error_response(StatusLine::new(404), method, &resource),
                }
            }
            _ => self.error_response(
                StatusLine::new(400),
                RequestMethod::Unknown,
                &Resource::borrowed(""),
            ),
        }
    }

    fn error_response(
        &self,
        status_line: StatusLine,
        method: RequestMethod,
        resource: &Resource,
    ) -> Response {
        match self
            .error_pages
            .iter()
            .find(|p| p.status_code == status_line.code)
        {
            Some(error_page) => error_page
                .response_type
                .to_response(method, resource)
                .unwrap_or_else(|status_line| Response::from(status_line)),
            None => Response::from(status_line),
        }
    }
}

impl StatusLine {
    pub fn new(code: u16) -> Self {
        StatusLine { code }
    }

    pub fn reason_phrase(&self) -> &'static str {
        // Ref: https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Status
        match self.code {
            200 => "OK",
            201 => "Created",
            301 => "Moved Permanently",
            302 => "Found",
            400 => "Bad Request",
            403 => "Forbidden",
            404 => "Not Found",
            405 => "Method Not Allowed",
            413 => "Content Too Large",
            414 => "URI Too Long",
            429 => "Too Many Requests",
            431 => "Request Header Fields Too Large",
            500 => "Internal Server Error",
            502 => "Bad Gateway",
            504 => "Gateway Timeout",
            505 => "HTTP Version Not Supported",
            _ => "",
        }
    }

    pub fn encode_http_str(&self) -> String {
        format!("HTTP/1.1 {} {}", self.code, self.reason_phrase())
    }
}

impl Response {
    pub fn new(status_line: StatusLine, body: String) -> Self {
        Response { status_line, body }
    }

    pub fn encode_http_str(&self) -> String {
        format!(
            concat!(
                "{}\n",
                "Content-Type: text/html; charset=utf-8\n",
                "Content-Length: {}\n",
                "\n",
                "{}"
            ),
            self.status_line.encode_http_str(),
            self.body.len(),
            self.body
        )
    }
}

impl From<StatusLine> for Response {
    fn from(status_line: StatusLine) -> Self {
        let body = String::from(status_line.reason_phrase());
        Response::new(status_line, body)
    }
}

impl ErrorPage {
    fn new(status_code: u16, response_type: ResponseType) -> Self {
        ErrorPage {
            status_code,
            response_type,
        }
    }

    pub fn file(status_code: u16, file_path: &'static str) -> Self {
        Self::new(status_code, ResponseType::new_file(file_path))
    }

    pub fn file_dynamic(status_code: u16, file_path: String) -> Self {
        Self::new(status_code, ResponseType::new_file_dynamic(file_path))
    }

    pub fn function<F>(status_code: u16, function: F) -> Self
    where
        F: Fn(RequestMethod, &Resource) -> Result<Response, StatusLine> + Send + Sync + 'static,
    {
        Self::new(status_code, ResponseType::new_function(function))
    }
}
