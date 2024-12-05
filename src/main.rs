use std::net::SocketAddr;
mod components;

use bytes::Bytes;
use components::composite_objects::{Path, RequestObject, ResponseObject};
use components::request_validator::validate_request;
use http_body_util::{combinators::BoxBody, BodyExt, Empty, Full};

use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use oas3::spec::Operation;
use oas3::{self, Spec};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    // We create a TcpListener and bind it to 127.0.0.1:3000
    let listener = TcpListener::bind(addr).await?;

    // We start a loop to continuously accept incoming connections
    loop {
        let (stream, _) = listener.accept().await?;

        // Use an adapter to access something implementing `tokio::io` traits as if they implement
        // `hyper::rt` IO traits.
        let io = TokioIo::new(stream);

        // Spawn a tokio task to serve multiple connections concurrently
        tokio::task::spawn(async move {
            // Finally, we bind the incoming connection to our `wchi` service
            if let Err(err) = http1::Builder::new()
                // `service_fn` converts our function in a `Service`
                .serve_connection(io, service_fn(echo))
                .await
            {
                eprintln!("Error serving connection: {:?}", err);
            }
        });
    }
}

async fn get_response_object_schema_by_operation(
    spec: &Spec,
    operation: &Operation,
) -> Option<ResponseObject> {
    let response = operation.responses(&spec);
    let content = response.get("200");

    let object_schema = match content {
        Some(a) => a
            .content
            .get("application/json")?
            .schema
            .as_ref()?
            .resolve(&spec)
            .ok(),
        None => None,
    };
    let response_object =
        ResponseObject::create_response_object_by_object_schema(&spec, &object_schema);
    return Some(response_object);
}

async fn get_request_object_schema_by_operation(
    spec: &Spec,
    operation: &Operation,
) -> Option<RequestObject> {
    let requests = match operation.clone().request_body {
        Some(rb) => rb.resolve(&spec),
        None => return None,
    };

    let object_schema = match requests {
        Ok(a) => a
            .content
            .get("application/json")?
            .schema
            .as_ref()?
            .resolve(&spec)
            .ok(),
        Err(_e) => None,
    };

    let request_object =
        RequestObject::create_request_object_by_object_schema(&spec, &object_schema);
    return Some(request_object);
}

async fn get_paths_from_spec(spec: &Spec) -> Vec<Path> {
    let path = match spec.paths.as_ref() {
        Some(p) => p,
        None => return vec![], // Return an empty vector if paths are None
    };

    let mut paths: Vec<Path> = vec![];

    // Iterate over paths using iter() to avoid ownership issues
    for (path_str, path_item) in path.iter() {
        if let Some(get) = path_item.get.as_ref() {
            // Call the asynchronous functions using `await`
            let response_object = get_response_object_schema_by_operation(&spec, get).await;
            let request_object = get_request_object_schema_by_operation(&spec, get).await;

            // Push the new Path into the vector
            paths.push(Path {
                path: path_str.clone(),
                method: Method::GET,
                response_object,
                request_object,
            });
        }

        if let Some(post) = path_item.post.as_ref() {
            // Call the asynchronous functions using `await`
            let response_object = get_response_object_schema_by_operation(&spec, post).await;
            let request_object = get_request_object_schema_by_operation(&spec, post).await;

            // Push the new Path into the vector
            paths.push(Path {
                path: path_str.clone(),
                method: Method::POST,
                response_object,
                request_object,
            });
        }
    }

    paths // Return the vector
}

async fn echo(
    req: Request<hyper::body::Incoming>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    let api_spec = match oas3::from_path("openapi.yaml") {
        Ok(spec) => Some(spec),
        Err(err) => {
            println!("Error in reading file, {}", err);
            None
        }
    }
    .unwrap();

    let paths: Vec<Path> = get_paths_from_spec(&api_spec).await;

    let req_method = req.method().clone();
    let req_path = req.uri().path().to_string();

    let body_bytes = req.collect().await?;
    let body = String::from_utf8(body_bytes.to_bytes().to_vec()).unwrap_or_else(|_| "".to_string());

    for p in paths.iter() {
        if req_method == &p.method && req_path == p.path.as_str() {
            if p.method == Method::POST {
                match validate_request(p.request_object.as_ref().unwrap(), body.clone()) {
                    Ok(_result) => (),
                    Err(error) => {
                        let response = Response::builder()
                            .status(StatusCode::OK)
                            .header("Content-Type", "application/json")
                            .body(full(error.error_message))
                            .unwrap();
                        return Ok(response);
                    }
                }
            }
            if let Some(response_object) = p.response_object.clone() {
                let response_string = response_object
                    .response
                    .unwrap_or_else(|| "No response".to_string());

                let response = Response::builder()
                    .status(StatusCode::OK)
                    .header("Content-Type", "application/json")
                    .body(full(response_string))
                    .unwrap();
                return Ok(response);
            }
        }
    }
    let mut not_found = Response::new(empty());
    *not_found.status_mut() = StatusCode::NOT_FOUND;
    Ok(not_found)
}
// We create some utility functions to make Empty and Full bodies
// fit our broadened Response body type.
fn empty() -> BoxBody<Bytes, hyper::Error> {
    Empty::<Bytes>::new()
        .map_err(|never| match never {})
        .boxed()
}
fn full<T: Into<Bytes>>(chunk: T) -> BoxBody<Bytes, hyper::Error> {
    Full::new(chunk.into())
        .map_err(|never| match never {})
        .boxed()
}
