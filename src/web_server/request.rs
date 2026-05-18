use std::{collections::BTreeMap, fmt::Display, str::FromStr, string::FromUtf8Error};
use urlencoding::{decode, encode};

use crate::web_server::json::Json;

pub struct Request {
    pub method: RequestMethod,
    pub resource: Resource,
    pub headers: Parameters,
    pub body: Body,
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

pub type Parameters = BTreeMap<String, String>;

pub struct Resource {
    pub path: String,
    pub query_params: Parameters,
}

pub enum Body {
    Text(String),
    FormData(Parameters),
    JsonData(Json),
}

impl Request {
    pub fn new(method: RequestMethod, resource: Resource, headers: Parameters, body: Body) -> Self {
        Request {
            method,
            resource,
            headers,
            body,
        }
    }

    pub fn to_log(&self) -> String {
        format!("{:?} {}", self.method, self.resource.to_log()) // don't want to log PI from headers or body
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
    fn new(path: String, query_params: Parameters) -> Self {
        Resource { path, query_params }
    }

    // TODO: Get original encoded value?
    pub fn url_decode(path: &str, query_params: Parameters) -> Result<Self, FromUtf8Error> {
        let path = decode(path)?.into_owned();

        let query_params = query_params
            .iter()
            .map(|(key, value)| Ok((decode(key)?.into_owned(), decode(value)?.into_owned())))
            .collect::<Result<Parameters, FromUtf8Error>>()?;

        Ok(Resource::new(path, query_params))
    }

    pub fn invalid() -> Self {
        Resource::new(String::new(), BTreeMap::new())
    }

    fn to_log(&self) -> String {
        let mut query_str = self
            .query_params
            .iter()
            .map(|(key, value)| format!("{}={}&", encode(key), encode(value)))
            .collect::<String>();

        // Remove trailing &
        if !query_str.is_empty() {
            query_str = format!("?{}", &query_str[0..query_str.len() - 1]);
        }

        let path = if Some("/") == self.path.get(0..1) {
            &self.path[1..]
        } else {
            &self.path
        };

        format!("/{}{}", encode(path), query_str)
    }
}

impl Display for Body {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Body::Text(s) => f.write_str(s),
            Body::FormData(params) => write!(f, "{:?}", params),
            Body::JsonData(json) => f.write_str(&json.to_string()),
        }
    }
}
