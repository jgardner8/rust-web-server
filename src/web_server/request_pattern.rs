use std::{borrow::Cow, fs};

use crate::web_server::{Request, RequestMethod, Resource, Response, StatusLine};

pub enum ResponseType {
    File(Cow<'static, str>),
    Function(Box<RequestProcessorFn>),
}

pub struct RequestPattern {
    method: RequestMethod,
    resource: Resource,
    response_type: ResponseType,
}

type RequestProcessorFn = dyn Fn(&Request) -> Result<Response, StatusLine> + Send + Sync;

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

    pub fn func<F>(method: RequestMethod, path: &'static str, function: F) -> Self
    where
        F: Fn(&Request) -> Result<Response, StatusLine> + Send + Sync + 'static,
    {
        RequestPattern::new(
            method,
            Resource::borrowed(path),
            ResponseType::new_func(function),
        )
    }

    pub fn function_dynamic<F>(method: RequestMethod, path: String, function: F) -> Self
    where
        F: Fn(&Request) -> Result<Response, StatusLine> + Send + Sync + 'static,
    {
        RequestPattern::new(
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
