use serde::{Deserialize, Serialize, Serializer};
use serde_json::{Map, Value};
use std::collections::HashMap;

use hyper::Method;
use oas3::{spec::ObjectSchema, Spec};

use crate::components::models::object_reference_handler::get_body_by_object_schema;

#[derive(Debug)]
pub struct PropertyValue {
    pub bool: Option<bool>,
    pub int: Option<i32>,
    pub number: Option<f32>,
    pub string: Option<String>,
    pub serde_value: Option<Value>,
}

impl Serialize for PropertyValue {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if let Some(b) = self.bool {
            return serializer.serialize_bool(b);
        }
        if let Some(f) = self.number {
            return serializer.serialize_f32(f);
        }
        if let Some(i) = self.int {
            return serializer.serialize_i32(i);
        }
        if let Some(ref s) = self.string {
            return serializer.serialize_str(s.as_str());
        }
        if let Some(ref s) = self.serde_value {
            return s.serialize(serializer);
        }
        return serializer.serialize_str("");
    }
}

#[derive(Debug, Clone)]
pub struct Path {
    pub request_object: Option<RequestObject>,
    pub response_object: Option<ResponseObject>,
    pub path: String,
    pub method: Method,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RequestObject {
    pub headers: Vec<String>,
    pub query_params: HashMap<String, String>,
    pub body: Option<String>,
}

impl RequestObject {
    pub fn init() -> RequestObject {
        return RequestObject {
            headers: vec![],
            query_params: HashMap::new(),
            body: None,
        };
    }

    pub fn create_request_object_by_object_schema(
        spec: &Spec,
        object_schema: &Option<ObjectSchema>,
    ) -> RequestObject {
        if let Some(obj) = object_schema {
            let body = get_body_by_object_schema(&spec, obj);
            let map = create_valid_json_from_hashmap(&body);

            return RequestObject {
                headers: vec![],
                query_params: HashMap::new(),
                body: Some(serde_json::to_string(&map).unwrap()),
            };
        } else {
            return RequestObject::init();
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ResponseObject {
    pub response: Option<String>,
    pub status_code: Option<i16>,
}

impl ResponseObject {
    pub fn create_response_object_by_object_schema(
        spec: &Spec,
        object_schema: &Option<ObjectSchema>,
    ) -> ResponseObject {
        if let Some(obj) = object_schema {
            let body = get_body_by_object_schema(&spec, obj);
            let map = create_valid_json_from_hashmap(&body);
            let ro = ResponseObject {
                response: Some(serde_json::to_string(&map).unwrap()),
                status_code: Some(200),
            };
            return ro;
        } else {
            return ResponseObject {
                response: None,
                status_code: None,
            };
        }
    }
}

fn insert_nested_value(root_map: &mut Map<String, Value>, keys: &[&str], value: &PropertyValue) {
    let mut key = keys[0];
    let mut is_array = false;
    if keys[0].ends_with("$array") {
        key = keys[0].split("$").next().unwrap();
        is_array = true;
    }
    if keys.len() == 1 {
        root_map.insert(String::from(key), serde_json::to_value(value).unwrap());
    } else {
        if is_array {
            let entry = root_map
                .entry(String::from(key))
                .or_insert_with(|| Value::Array(vec![Value::Object(Map::new())]));

            let nested_array = entry.as_array_mut().unwrap().get_mut(0).unwrap();
            if let Value::Object(x) = nested_array {
                insert_nested_value(x, &keys[1..], value);
            }
        } else {
            let entry = root_map
                .entry(String::from(key))
                .or_insert_with(|| Value::Object(Map::new()));

            if let Value::Object(nested_map) = entry {
                insert_nested_value(nested_map, &keys[1..], value);
            }
        }
    }
}

fn create_valid_json_from_hashmap(map: &HashMap<String, PropertyValue>) -> Map<String, Value> {
    let mut root = Map::new();
    for (key, value) in map {
        let keys: Vec<&str> = key.split('.').collect();
        insert_nested_value(&mut root, &keys, value);
    }
    root
}
