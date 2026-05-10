use std::{borrow::Cow, fs};

use crate::web_server::{Request, RequestMethod, Resource, Response, StatusLine};

pub struct Route {
    method: RequestMethod,
    resource: Resource,
    response_type: ResponseType,
}

pub struct ErrorRoute {
    status_code: u16,
    response_type: ResponseType,
}

pub enum ResponseType {
    File(Cow<'static, str>),
    Function(Box<RequestProcessorFn>),
}

type RequestProcessorFn = dyn Fn(&Request) -> Result<Response, StatusLine> + Send + Sync;

impl Route {
    fn new(method: RequestMethod, resource: Resource, response_type: ResponseType) -> Self {
        Route {
            method,
            resource,
            response_type,
        }
    }

    pub fn file(method: RequestMethod, path: &'static str, file_path: &'static str) -> Self {
        Route::new(
            method,
            Resource::borrowed(path),
            ResponseType::new_file(file_path),
        )
    }

    pub fn file_dynamic(method: RequestMethod, path: String, file_path: String) -> Self {
        Route::new(
            method,
            Resource::owned(path),
            ResponseType::new_file_dynamic(file_path),
        )
    }

    pub fn func<F>(method: RequestMethod, path: &'static str, function: F) -> Self
    where
        F: Fn(&Request) -> Result<Response, StatusLine> + Send + Sync + 'static,
    {
        Route::new(
            method,
            Resource::borrowed(path),
            ResponseType::new_func(function),
        )
    }

    pub fn func_dynamic<F>(method: RequestMethod, path: String, function: F) -> Self
    where
        F: Fn(&Request) -> Result<Response, StatusLine> + Send + Sync + 'static,
    {
        Route::new(
            method,
            Resource::owned(path),
            ResponseType::new_func(function),
        )
    }

    pub fn matches_path(&self, path_no_query_params: &str) -> bool {
        self.resource.path == path_no_query_params
    }

    pub fn matches(&self, method: RequestMethod, path_no_query_params: &str) -> bool {
        self.method == method && self.resource.path == path_no_query_params
    }

    pub fn to_response(&self, request: &Request) -> Result<Response, StatusLine> {
        self.response_type.to_response(request)
    }
}

impl ErrorRoute {
    fn new(status_code: u16, response_type: ResponseType) -> Self {
        ErrorRoute {
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
        F: Fn(&Request) -> Result<Response, StatusLine> + Send + Sync + 'static,
    {
        Self::new(status_code, ResponseType::new_func(function))
    }

    pub fn matches(&self, status_code: u16) -> bool {
        self.status_code == status_code
    }

    pub fn to_response(&self, request: &Request) -> Result<Response, StatusLine> {
        self.response_type.to_response(request)
    }
}

impl ResponseType {
    pub fn new_file(path: &'static str) -> ResponseType {
        ResponseType::File(Cow::Borrowed(path))
    }

    pub fn new_file_dynamic(path: String) -> ResponseType {
        ResponseType::File(Cow::Owned(path))
    }

    pub fn new_func<F>(function: F) -> ResponseType
    where
        F: Fn(&Request) -> Result<Response, StatusLine> + Send + Sync + 'static,
    {
        ResponseType::Function(Box::new(function))
    }

    pub fn to_response(&self, request: &Request) -> Result<Response, StatusLine> {
        match self {
            ResponseType::File(path) => self.file_response(path),
            ResponseType::Function(f) => f(request),
        }
    }

    fn file_response(&self, path: &str) -> Result<Response, StatusLine> {
        match fs::read_to_string(path) {
            Ok(response_body) => Ok(Response::new(200, response_body)),
            Err(e) => {
                eprintln!("Error: Cannot read file {}, error: {:?}", path, e);
                Err(StatusLine::new(404))
            }
        }
    }
}
