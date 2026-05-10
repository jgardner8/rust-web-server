use std::{borrow::Cow, collections::BTreeMap, str::FromStr};

#[derive(Debug)]
pub struct Request {
    pub method: RequestMethod,
    pub resource: Resource,
    pub headers: BTreeMap<String, String>,
    pub body: String,
}

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

#[derive(Debug)]
pub struct Resource {
    pub path: Cow<'static, str>,
}

impl Request {
    pub fn new(
        method: RequestMethod,
        resource: Resource,
        headers: BTreeMap<String, String>,
        body: String,
    ) -> Self {
        Request {
            method,
            resource,
            headers,
            body,
        }
    }
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

    pub fn invalid() -> Self {
        Resource::borrowed("")
    }
}
