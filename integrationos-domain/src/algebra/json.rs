use serde_json::Value;

pub trait JsonExt {
    fn drop_nulls(&self) -> Self;
}

impl JsonExt for Value {
    fn drop_nulls(&self) -> Value {
        remove_nulls(self)
    }
}

fn remove_nulls(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut map = map.clone();

            let keys_to_remove: Vec<String> = map
                .iter()
                .filter(|(_, v)| v.is_null())
                .map(|(k, _)| k.clone())
                .collect();

            for key in keys_to_remove {
                map.remove(&key);
            }

            for value in map.values_mut() {
                *value = remove_nulls(value);
            }

            Value::Object(map)
        }
        Value::Array(vec) => {
            let mut vec = vec.clone();

            for item in vec.iter_mut() {
                *item = remove_nulls(item);
            }

            Value::Array(vec)
        }
        _ => value.clone(),
    }
}
