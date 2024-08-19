use super::{
    common_model::{CommonModel, DataType, Expandable},
    json_mapper::Field,
};
use crate::{IntegrationOSError, InternalError};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Default)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
pub struct JsonSchema {
    #[serde(rename = "type")]
    pub type_name: String,
    #[serde(default = "HashMap::new")]
    pub properties: HashMap<String, Property>,
    pub required: Option<Vec<String>>,
    pub path: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<Property>>,
}

impl JsonSchema {
    pub fn new(type_name: String) -> Self {
        Self {
            type_name,
            properties: HashMap::new(),
            required: None,
            path: None,
            items: None,
        }
    }

    pub fn empty() -> Self {
        Self {
            type_name: "object".to_string(),
            properties: HashMap::new(),
            required: None,
            path: None,
            items: None,
        }
    }

    pub fn from_value(value: Value) -> Result<Self, IntegrationOSError> {
        serde_json::from_value::<Self>(value.clone())
            .map_err(|e| InternalError::invalid_argument(&e.to_string(), Some(&value.to_string())))
    }

    pub fn to_value(&self) -> Result<Value, IntegrationOSError> {
        serde_json::to_value(self)
            .map_err(|e| InternalError::invalid_argument(&e.to_string(), None))
    }

    pub fn filter(mut self, keys_to_remove: &[String]) -> Self {
        self.properties.retain(|name, _| {
            let retain = !keys_to_remove.contains(name);

            if !retain {
                if let Some(ref mut required) = self.required {
                    required.retain(|n| n != name);
                }
            }

            retain
        });
        self
    }

    pub fn keys_at_path(&self, search_path: &str) -> Vec<String> {
        if search_path == "$" {
            return self.properties.keys().cloned().collect();
        }

        self.properties
            .iter()
            .flat_map(|(_, property)| self.collect_keys(property, search_path))
            .collect()
    }

    pub fn keys(&self) -> String {
        self.properties
            .keys()
            .cloned()
            .collect::<Vec<String>>()
            .join(", ")
    }

    #[allow(clippy::only_used_in_recursion)]
    fn collect_keys(&self, property: &Property, search_path: &str) -> Vec<String> {
        let mut keys = vec![];

        if let Some(ref actual_path) = property.path {
            if actual_path == search_path {
                if let Some(nested_properties) = &property.properties {
                    keys.extend(nested_properties.keys().cloned());
                }
                if let Some(nested_items) = &property.items {
                    if let Some(nested_properties) = &nested_items.properties {
                        keys.extend(nested_properties.keys().cloned());
                    }
                }
                return keys;
            }
        }

        if let Some(nested_properties) = &property.properties {
            keys.extend(
                nested_properties
                    .iter()
                    .flat_map(|(_, nested_property)| {
                        self.collect_keys(nested_property, search_path)
                    })
                    .collect::<Vec<String>>(),
            );
        }
        if let Some(nested_items) = &property.items {
            keys.extend(self.collect_keys(nested_items, search_path));
        }

        keys
    }

    pub fn remove_expandables(mut self) -> JsonSchema {
        self.properties.retain(|name, value| {
            let retain = !matches!(value.r#type.as_str(), "array" | "object");

            if !retain {
                if let Some(ref mut required) = self.required {
                    required.retain(|n| n != name);
                }
            }

            retain
        });
        self
    }

    pub fn remove_primitives(mut self) -> JsonSchema {
        self.properties.retain(|name, value| {
            let retain = matches!(value.r#type.as_str(), "array" | "object");

            if !retain {
                if let Some(ref mut required) = self.required {
                    required.retain(|n| n != name);
                }
            }

            retain
        });
        self
    }

    pub fn flatten(mut self) -> JsonSchema {
        self.properties
            .iter_mut()
            .for_each(|(_, value)| match value.r#type.as_str() {
                "array" => {
                    value.properties = None;
                    value.items = None;
                }
                "object" => {
                    value.properties = None;
                    value.items = None;
                }
                _ => {}
            });

        self
    }

    pub fn extract_expandables(&self) -> Vec<JsonSchema> {
        let mut schemas = vec![];

        for (k, v) in &self.properties {
            let path = format!("$.{k}");

            match v.r#type.as_str() {
                "array" => schemas.push(JsonSchema {
                    type_name: v.r#type.clone(),
                    properties: v.properties.clone().unwrap_or_default(),
                    required: None,
                    path: Some(path),
                    items: None,
                }),
                "object" => schemas.push(JsonSchema {
                    type_name: v.r#type.clone(),
                    properties: v.properties.clone().unwrap_or_default(),
                    required: None,
                    path: Some(path),
                    items: None,
                }),
                _ => {}
            }
        }

        schemas
    }

    pub fn insert(&mut self, name: String, r#type: String, path: String) {
        self.properties.insert(
            name,
            Property {
                r#type,
                path: Some(path),
                description: None,
                properties: None,
                items: None,
                r#enum: None,
            },
        );
    }
}

impl TryFrom<CommonModel> for JsonSchema {
    type Error = IntegrationOSError;

    fn try_from(common_model: CommonModel) -> std::prelude::v1::Result<Self, Self::Error> {
        let mut properties = HashMap::new();
        for field in common_model.fields {
            properties.insert(field.name, field.datatype.try_into()?);
        }

        Ok(JsonSchema {
            type_name: "object".to_string(),
            properties,
            required: None,
            path: None,
            items: None,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
pub struct Property {
    #[serde(rename = "type")]
    pub r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "dummy", dummy(default))]
    pub properties: Option<HashMap<String, Property>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "dummy", dummy(default))]
    pub items: Option<Box<Property>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "dummy", dummy(default))]
    pub r#enum: Option<Vec<String>>,
}

impl Property {
    pub fn new(r#type: &str, desc: Option<&str>) -> Self {
        Self {
            r#type: r#type.to_string(),
            path: None,
            description: desc.map(|d| d.to_string()),
            properties: None,
            items: None,
            r#enum: None,
        }
    }

    pub fn retain_recursive(&mut self, name: &str, map: &HashMap<String, Field>) -> bool {
        match self.r#type.as_str() {
            "object" => {
                let Some(ref mut props) = self.properties else {
                    return true;
                };
                props.retain(|sub_name, prop| {
                    if let Some(Field::Object { fields, .. }) = map.get(name) {
                        prop.retain_recursive(sub_name, fields)
                    } else if let Some(Field::Array { items, .. }) = map.get(name) {
                        if let Field::Object { fields, .. } = items.as_ref() {
                            prop.retain_recursive(sub_name, fields)
                        } else {
                            true
                        }
                    } else {
                        true
                    }
                });
                !props.is_empty()
            }
            "array" => {
                if let Some(ref mut items) = self.items {
                    items.retain_recursive(name, map)
                } else {
                    true
                }
            }
            _ => !map.contains_key(name),
        }
    }
}

impl TryFrom<DataType> for Property {
    type Error = IntegrationOSError;

    fn try_from(data_type: DataType) -> std::prelude::v1::Result<Self, Self::Error> {
        match data_type {
            DataType::String => Ok(Property::new("string", None)),
            DataType::Number => Ok(Property::new("number", None)),
            DataType::Boolean => Ok(Property::new("boolean", None)),
            DataType::Date => Ok(Property::new("number", None)),
            DataType::Enum { options, .. } => {
                let options = options
                    .unwrap_or_default()
                    .into_iter()
                    .map(|o| o.to_string())
                    .collect::<Vec<String>>();
                Ok(Property {
                    r#type: "string".to_string(),
                    path: None,
                    description: None,
                    properties: None,
                    items: None,
                    r#enum: Some(options),
                })
            }

            DataType::Expandable(expandable) => match expandable {
                Expandable::Expanded { model, .. } => {
                    let mut map = HashMap::new();
                    for field in model.fields {
                        map.insert(field.name, field.datatype.try_into()?);
                    }
                    Ok(Property {
                        r#type: "object".to_string(),
                        path: None,
                        description: None,
                        properties: Some(map),
                        items: None,
                        r#enum: None,
                    })
                }
                _ => Ok(Property {
                    r#type: "object".to_string(),
                    path: None,
                    description: None,
                    properties: None,
                    items: None,
                    r#enum: None,
                }),
            },
            DataType::Array { element_type } => Ok(Property {
                r#type: "array".to_string(),
                path: None,
                description: None,
                properties: None,
                items: Some(Box::new(Property::try_from(*element_type)?)),
                r#enum: None,
            }),
            DataType::Unknown => Ok(Property {
                r#type: "unknown".to_string(),
                path: None,
                description: None,
                properties: None,
                items: None,
                r#enum: None,
            }),
        }
    }
}

pub fn generate_schema(input: &Value, json_path: &str) -> Value {
    match input {
        Value::Object(map) => {
            let mut properties: Map<String, Value> = Map::new();

            for (key, value) in map.iter() {
                let new_path = format!("{}.{}", json_path, key);
                properties.insert(key.to_string(), generate_value_schema(value, &new_path));
            }

            json!({
                "type": "object",
                "path": json_path,
                "properties": properties,
            })
        }
        Value::Array(arr) => {
            let item_schema = if let Some(item) = arr.first() {
                generate_value_schema(item, &format!("{}[0]", json_path))
            } else {
                json!({ "type": "unknown", "path": format!("{}[0]", json_path) })
            };

            json!({
                "type": "array",
                "path": json_path,
                "items": item_schema,
            })
        }
        _ => json!({
            "type": "unknown",
            "path": json_path,
        }),
    }
}

pub fn extract_flat_primitive_keys(input: &Value) -> Value {
    // Initialize a JSON Schema object
    let mut properties: Map<String, Value> = Map::new();

    // Iterate over the object's keys and values if it's an object
    if let Value::Object(map) = input {
        for (key, value) in map.iter() {
            // Determine the type based on the JSON value
            let type_name = match value {
                Value::String(_) => "string",
                Value::Number(_) => "number",
                Value::Bool(_) => "boolean",
                Value::Null => "unknown",
                _ => continue, // Skip non-primitive types
            };

            // Add the property to the schema
            properties.insert(
                key.to_string(),
                json!({ "type": type_name, "path": format!("$.{}", key) }),
            );
        }
    }

    // Return the final JSON Schema object
    json!({
        "type": "object",
        "properties": properties,
    })
}

pub fn extract_nested_keys(input: &Value, json_path: &str) -> Value {
    let mut properties: Map<String, Value> = Map::new();

    if let Value::Object(map) = input {
        for (key, value) in map.iter() {
            let new_path = format!("{}.{}", json_path, key);

            // Check for objects or arrays
            match value {
                Value::Object(_) => {
                    properties.insert(
                        key.to_string(),
                        json!({ "type": "object", "path": new_path }),
                    );
                }
                Value::Array(_) => {
                    properties.insert(
                        key.to_string(),
                        json!({ "type": "array", "path": new_path }),
                    );
                }
                _ => continue,
            };
        }
    }

    json!({
        "type": "object",
        "properties": properties,
    })
}

pub fn generate_value_schema(value: &Value, json_path: &str) -> Value {
    match value {
        Value::String(_) => json!({ "type": "string", "path": json_path }),
        Value::Number(_) => json!({ "type": "number", "path": json_path }),
        Value::Bool(_) => json!({ "type": "boolean", "path": json_path }),
        Value::Null => json!({ "type": "unknown", "path": json_path }),
        Value::Object(_) => generate_schema(value, json_path),
        Value::Array(arr) => {
            let item_schema = if let Some(item) = arr.first() {
                generate_value_schema(item, &format!("{}[0]", json_path))
            } else {
                json!({ "type": "unknown", "path": format!("{}[0]", json_path) })
            };

            json!({
                "type": "array",
                "path": json_path,
                "items": item_schema,
            })
        }
    }
}

#[cfg(all(test, not(feature = "notest")))]
mod tests {
    use super::*;

    use serde_json::json;
    use tracing::{info, metadata::LevelFilter};
    use tracing_subscriber::EnvFilter;

    #[ignore]
    #[test]
    fn test_keys_at_path() {
        let filter = EnvFilter::builder()
            .with_default_directive(LevelFilter::DEBUG.into())
            .from_env_lossy();
        tracing_subscriber::fmt().with_env_filter(filter).init();

        let schema_json = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string", "path": "$.name" },
                "age": { "type": "number", "path": "$.age" },
                "email": { "type": "string", "path": "$.email" },
                "address": {
                    "type": "object",
                    "path": "$.address",
                    "properties": {
                        "street": { "type": "string", "path": "$.address.street" },
                        "city": { "type": "string", "path": "$.address.city" },
                        "state": { "type": "string", "path": "$.address.state" },
                        "postalCode": { "type": "string", "path": "$.address.postalCode" }
                    }
                },
                "phoneNumbers": {
                    "type": "array",
                    "path": "$.phoneNumbers",
                    "items": {
                        "type": "object",
                        "path": "$.phoneNumbers",
                        "properties": {
                            "type": { "type": "string", "path": "$.phoneNumbers.type" },
                            "number": { "type": "string", "path": "$.phoneNumbers.number" }
                        }
                    }
                },
                "emails": {
                    "type": "array",
                    "path": "$.emails",
                    "items": {
                        "type": "string",
                        "path": "$.emails[0]"
                    }
                }
            }
        });

        // Deserialize the JSON value into a JsonSchema instance
        let schema: JsonSchema = serde_json::from_value(schema_json).unwrap();

        let path_to_search = "$";
        let keys = schema.keys_at_path(path_to_search);

        info!("Keys: {:#?}", keys);

        // Check that the keys for the given path are returned
        // assert_eq!(keys, vec!["name".to_string(), "age".to_string()]);
    }

    #[ignore]
    #[test]
    fn test_generate_schema_on_object() {
        let filter = EnvFilter::builder()
            .with_default_directive(LevelFilter::DEBUG.into())
            .from_env_lossy();
        tracing_subscriber::fmt().with_env_filter(filter).init();

        let input = json!({
            "name": "John",
            "profile": { "city": "New York", "age": 25 },
            "scores": [10, 20, 30],
            "address": {
                "city": "New York",
                "postalCodes": [10001, 10002]
            },
            "phoneNumbers": [
                {
                    "type": "home",
                    "number": "212 555-1234"
                },
                {
                    "type": "office",
                    "number": "646 555-4567"
                }
            ],
            "nullValue": null,
            "emptyArray": [],
        });

        let json_path = "$";

        let result = generate_schema(&input, json_path);
        info!("result: {:#?}", result);

        // assert_eq!(result, expected_output);
    }

    #[ignore]
    #[test]
    fn test_generate_schema_on_array() {
        let filter = EnvFilter::builder()
            .with_default_directive(LevelFilter::DEBUG.into())
            .from_env_lossy();
        tracing_subscriber::fmt().with_env_filter(filter).init();

        let input = json!([{
            "channel_id": 1,
            "enabled_currencies": [
                "USD"
            ],
            "default_currency": "USD",
            "meta": {
                "responseJSON": {
                    "data": [
                        {
                            "channel_id": 1,
                            "enabled_currencies": [
                                "USD"
                            ],
                            "default_currency": "USD"
                        },
                        {
                            "channel_id": 664177,
                            "enabled_currencies": [
                                "USD",
                                "GBP"
                            ],
                            "default_currency": "USD"
                        },
                        {
                            "channel_id": 664179,
                            "enabled_currencies": [
                                "USD",
                                "AUD"
                            ],
                            "default_currency": "USD"
                        },
                        {
                            "channel_id": 667159,
                            "enabled_currencies": [
                                "USD"
                            ],
                            "default_currency": "USD"
                        }
                    ]
                }
            }
        }]);

        let json_path = "$";

        let result = generate_schema(&input, json_path);
        info!("result: {:#?}", result);

        // assert_eq!(result, expected_output);
    }

    #[ignore]
    #[test]
    fn test_extract_flat_primitive_keys() {
        let filter = EnvFilter::builder()
            .with_default_directive(LevelFilter::DEBUG.into())
            .from_env_lossy();
        tracing_subscriber::fmt().with_env_filter(filter).init();

        let input = json!({
            "name": "John",
            "age": 30,
            "is_student": false,
            "score": null,
            "address": {
                "city": "New York",
                "postalCodes": [10001, 10002]
            },
        });

        let _expected_output = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string", "path": "$.name" },
                "age": { "type": "number", "path": "$.age" },
                "is_student": { "type": "boolean", "path": "$.is_student" },
                "score": { "type": "null", "path": "$.score" },
            }
        });

        let result = extract_flat_primitive_keys(&input);
        info!("result: {:#?}", result);
    }

    #[ignore]
    #[test]
    fn test_extract_nested_keys() {
        let filter = EnvFilter::builder()
            .with_default_directive(LevelFilter::DEBUG.into())
            .from_env_lossy();
        tracing_subscriber::fmt().with_env_filter(filter).init();

        let input = json!({
            "name": "John",
            "profile": { "city": "New York" },
            "scores": [10, 20, 30]
        });

        let json_path = "$";

        let _expected_output = json!({
            "type": "object",
            "properties": {
                "profile": { "type": "object", "path": "$.profile" },
                "scores": { "type": "array", "path": "$.scores" }
            }
        });

        let result = extract_nested_keys(&input, json_path);
        info!("result: {:#?}", result);
    }

    #[test]
    fn test_schemars() {
        use schemars::schema_for;
        use schemars::JsonSchema;

        #[derive(Debug, Serialize, Deserialize, JsonSchema)]
        struct Response {
            /// The map array for all fields
            map: Vec<Map>,

            /// The comments for the map
            comments: String,

            /// The potential issues with the map
            potential_issues: String,
        }

        #[derive(Debug, Serialize, Deserialize, JsonSchema)]
        struct Map {
            /// The name of the field in the source model
            source_field_name: String,

            /// The name of the field in the destination model, empty if no match found
            destination_field_name: String,

            /// Whether a match was found or not
            match_found: bool,

            /// The confidence score, a number between 0 and 1.
            confidence_score: f64,

            /// The transformation function needed, if not needed then identity.
            source_to_destination_transformation: String,

            /// The transformation function needed, if not needed then identity.
            destination_to_source_transformation: String,

            /// The reasoning for the match
            reasoning: String,

            /// The potential issues with this mapping when confidenceScore is low, this could be empty, or a text to explain potential issues with the map or the transformation function
            potential_issues: String,
        }

        let schema = schema_for!(Response);

        println!("{:#?}", serde_json::to_value(&schema).unwrap());
    }
}
