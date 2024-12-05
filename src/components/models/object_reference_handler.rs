use crate::components::composite_objects::PropertyValue;
use oas3::{
    spec::{ObjectSchema, SchemaType, SchemaTypeSet},
    Spec,
};
use rand::Rng;
use serde_json::Value;
use std::{any::Any, collections::HashMap};

pub fn get_body_by_object_schema(
    spec: &Spec,
    obj_original: &ObjectSchema,
) -> HashMap<String, PropertyValue> {
    let mut hash_map = HashMap::new();
    let mut stack: Vec<(String, ObjectSchema)> = Vec::new();

    // Start with the original schema, add it to the stack with an empty key
    stack.push(("".to_string(), obj_original.clone()));

    while let Some((key_prefix, obj)) = stack.pop() {
        let properties = &obj.properties;

        // If there are no properties, check if it's an array
        if properties.is_empty() {
            if is_single_property(&obj) {
                let prop_value = get_values_from_property(
                    obj.schema_type.as_ref().unwrap(),
                    obj.example,
                    &obj.enum_values,
                );
                hash_map.insert(
                    key_prefix.clone(),
                    get_property_value_object(&prop_value.unwrap()),
                );
            }
            if obj
                .schema_type
                .as_ref()
                .unwrap()
                .is_array_or_nullable_array()
            {
                if let Some(items_ref) = obj.items {
                    let items_resolved = items_ref.resolve(&spec).unwrap();
                    stack.push((key_prefix + "$array", items_resolved));
                }
            }

            continue;
        }

        // Iterate through properties and add them to the stack or hashmap
        for (k, v) in properties.iter() {
            let property_object = v.resolve(&spec).unwrap();
            if !obj.required.contains(&k) {
                continue;
            }
            let full_key = if key_prefix.is_empty() {
                k.clone()
            } else {
                format!("{}.{}", key_prefix, k)
            };

            if is_single_property(&property_object) {
                let type_set = property_object.schema_type.unwrap();
                if let Some(value) =
                    get_values_from_property(&type_set, property_object.example, &obj.enum_values)
                {
                    hash_map.insert(full_key, get_property_value_object(&value));
                }
            } else {
                // If it's not a single property, add it to the stack to process later
                stack.push((full_key, property_object));
            }
        }
    }

    hash_map
}

fn get_property_value_object(value: &Box<dyn Any>) -> PropertyValue {
    if let Some(string_val) = value.downcast_ref::<String>() {
        PropertyValue {
            bool: None,
            number: None,
            int: None,
            string: Some(string_val.clone()), // Clone the String
            serde_value: None,
        }
    } else if let Some(int_val) = value.downcast_ref::<i32>() {
        PropertyValue {
            bool: None,
            number: None,
            int: Some(*int_val), // Dereference the i16 (copy)
            string: None,
            serde_value: None,
        }
    } else if let Some(float_val) = value.downcast_ref::<f32>() {
        PropertyValue {
            bool: None,
            number: Some(*float_val), // Dereference the f32 (copy)
            int: None,
            string: None,
            serde_value: None,
        }
    } else if let Some(bool_val) = value.downcast_ref::<bool>() {
        PropertyValue {
            bool: Some(*bool_val), // Dereference the bool (copy)
            number: None,
            int: None,
            string: None,
            serde_value: None,
        }
    } else if let Some(val_value) = value.downcast_ref::<Value>() {
        PropertyValue {
            bool: None, // Dereference the bool (copy)
            number: None,
            int: None,
            string: None,
            serde_value: Some(val_value.clone()),
        }
    } else {
        PropertyValue {
            bool: None,
            number: None,
            int: None,
            string: Some(String::from("Unknown")), // Return a default String for unknown types
            serde_value: None,
        }
    }
}

fn is_single_property(object_schema: &ObjectSchema) -> bool {
    let schema_type = object_schema.schema_type.as_ref().unwrap();
    return schema_type.contains(SchemaType::String)
        || schema_type.contains(SchemaType::Integer)
        || schema_type.contains(SchemaType::Number)
        || schema_type.contains(SchemaType::Boolean);
}

// Updated get_values_from_property function to return Box<dyn Any>
fn get_values_from_property(
    type_set: &SchemaTypeSet,
    example: Option<serde_json::Value>,
    enum_values: &Vec<String>,
) -> Option<Box<dyn Any>> {
    if let Some(e) = example {
        return Some(Box::new(e));
    }
    if !enum_values.is_empty() {
        let mut rng = rand::thread_rng();
        let random = rng.gen_range(0..enum_values.len() - 1);
        return Some(Box::new(enum_values[random].clone()));
    }

    match type_set {
        SchemaTypeSet::Single(SchemaType::Boolean) => Some(Box::new(true)),
        SchemaTypeSet::Single(SchemaType::String) => Some(Box::new("asd".to_string())),
        SchemaTypeSet::Single(SchemaType::Number) => Some(Box::new(1.2)),
        SchemaTypeSet::Single(SchemaType::Integer) => Some(Box::new(1)),
        _ => None,
    }
}
