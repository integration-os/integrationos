use super::{
    common_model::{DataType, Expandable},
    json_schema::Property,
};
use crate::{IntegrationOSError, InternalError};
use jsonpath_lib::select;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Deserialize, Debug, Clone, Serialize, Eq, PartialEq)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
#[serde(tag = "type")]
pub enum Field {
    #[serde(rename = "string")]
    String {
        path: String,
        transformation: String,
        required: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        default: Option<FieldDefault>,
    },
    #[serde(rename = "boolean")]
    Boolean {
        path: String,
        transformation: String,
        required: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        default: Option<FieldDefault>,
    },
    #[serde(rename = "number")]
    Number {
        path: String,
        transformation: String,
        required: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        default: Option<FieldDefault>,
    },
    #[serde(rename = "object")]
    Object {
        required: bool,
        fields: HashMap<String, Field>,
    },
    #[serde(rename = "array")]
    Array {
        path: String,
        required: bool,
        items: Box<Field>,
    },
}

impl Field {
    pub fn from_property(
        property: &Property,
        path: String,
        transformation: String,
        required: bool,
    ) -> Result<Self, IntegrationOSError> {
        let field = match property.r#type.as_str() {
            "string" => Field::String {
                path,
                transformation,
                required,
                default: None,
            },
            "number" => Field::Number {
                path,
                transformation,
                required,
                default: None,
            },
            "boolean" => Field::Boolean {
                path,
                transformation,
                required,
                default: None,
            },
            "object" => {
                let Some(ref properties) = property.properties else {
                    return Err(InternalError::configuration_error(
                        "No properties in field object",
                        None,
                    ));
                };
                let mut fields = HashMap::new();
                for (name, prop) in properties {
                    let field =
                        Self::from_property(prop, path.clone(), transformation.clone(), required)?;
                    fields.insert(name.to_owned(), field);
                }
                Field::Object { required, fields }
            }
            "array" => {
                let Some(ref items) = property.items else {
                    return Err(InternalError::configuration_error(
                        "No items in field array",
                        None,
                    ));
                };
                Field::Array {
                    path: path.clone(),
                    required,
                    items: Box::new(Self::from_property(items, path, transformation, required)?),
                }
            }
            "unknown" => Field::Object {
                required,
                fields: HashMap::new(),
            },
            _ => {
                return Err(InternalError::configuration_error(
                    &format!("Invalid source field type: {}", property.r#type.as_str()),
                    None,
                ))
            }
        };
        Ok(field)
    }

    pub fn from_data_type(
        data_type: &DataType,
        path: String,
        transformation: String,
        required: bool,
    ) -> Result<Self, IntegrationOSError> {
        let field = match data_type {
            DataType::String => Field::String {
                path,
                transformation,
                required,
                default: None,
            },
            DataType::Number => Field::Number {
                path,
                transformation,
                required,
                default: None,
            },
            DataType::Boolean => Field::Boolean {
                path,
                transformation,
                required,
                default: None,
            },
            DataType::Date => Field::Number {
                path,
                transformation,
                required,
                default: None,
            },
            DataType::Enum { .. } => Field::String {
                path,
                transformation,
                required,
                default: None,
            },
            DataType::Expandable(e) => {
                let Expandable::Expanded { model, .. } = e else {
                    return Err(InternalError::configuration_error(
                        "Expandable is unexpanded",
                        None,
                    ));
                };

                let mut fields = HashMap::new();
                for field in &model.fields {
                    let name = field.name.clone();
                    let field = Self::from_data_type(
                        &field.datatype,
                        path.clone(),
                        transformation.clone(),
                        required,
                    )?;
                    fields.insert(name, field);
                }
                Field::Object { required, fields }
            }
            DataType::Array { element_type } => Field::Array {
                path: path.clone(),
                required,
                items: Box::new(Self::from_data_type(
                    element_type,
                    path,
                    transformation,
                    required,
                )?),
            },
        };
        Ok(field)
    }

    pub fn prepend_path(&mut self, new_path: &str) {
        let path = match self {
            Field::String { path, .. } => path,
            Field::Boolean { path, .. } => path,
            Field::Number { path, .. } => path,
            Field::Object { fields, .. } => {
                for field in fields.values_mut() {
                    field.prepend_path(new_path);
                }
                return;
            }
            Field::Array { items, .. } => {
                items.prepend_path(new_path);
                return;
            }
        };
        if new_path.is_empty() {
            *path = format!("$.{path}");
        } else {
            *path = format!("$.{new_path}.{path}");
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
pub struct FieldDefault {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<String>,
}

pub type SchemaMappingDefinition = HashMap<String, Field>;

pub fn map_data_by_schema(
    data: &Value,
    config: &SchemaMappingDefinition,
) -> Result<Value, IntegrationOSError> {
    fn string(
        data: &Value,
        key: &str,
        path: &str,
        required: bool,
        default: &Option<FieldDefault>,
    ) -> Result<Value, IntegrationOSError> {
        if let Some(selected_value) = select(data, path)
            .ok()
            .and_then(|selected| selected.into_iter().next())
        {
            return Ok(selected_value.clone());
        }

        if let Some(FieldDefault { value, .. }) = default {
            return Ok(Value::String(value.clone().unwrap_or_default()));
        }

        if required {
            return Err(InternalError::configuration_error(
                &format!("Missing required field to decode as string: {}", key),
                None,
            ));
        }

        Ok(Value::Null)
    }

    fn boolean(
        data: &Value,
        key: &str,
        path: &str,
        required: bool,
        default: &Option<FieldDefault>,
    ) -> Result<Value, IntegrationOSError> {
        if let Some(selected_value) = select(data, path)
            .ok()
            .and_then(|selected| selected.into_iter().next())
        {
            if let Value::Null = &selected_value {
                return Ok(selected_value.clone());
            }

            if let Value::Bool(b) = &selected_value {
                return Ok(Value::Bool(*b));
            } else {
                return Err(InternalError::configuration_error(
                    &format!("Invalid data type for boolean field: {}", key),
                    None,
                ));
            }
        }

        if let Some(FieldDefault { value, .. }) = default {
            let default_bool = value
                .as_deref()
                .unwrap_or("false")
                .parse::<bool>()
                .map(Value::Bool);

            if let Ok(default_bool) = default_bool {
                return Ok(default_bool);
            } else {
                return Err(InternalError::configuration_error(
                    &format!("Invalid default value for boolean field: {}", key),
                    None,
                ));
            }
        }

        if required {
            return Err(InternalError::configuration_error(
                &format!("Missing required field to decode as boolean: {}", key),
                None,
            ));
        }

        Ok(Value::Null)
    }

    fn number(
        data: &Value,
        key: &str,
        path: &str,
        required: bool,
        default: &Option<FieldDefault>,
    ) -> Result<Value, IntegrationOSError> {
        if let Some(selected_value) = select(data, path)
            .ok()
            .and_then(|selected| selected.into_iter().next())
        {
            if let Value::Null = &selected_value {
                return Ok(selected_value.clone());
            }

            if let Value::Number(n) = &selected_value {
                return Ok(Value::Number(n.clone()));
            } else {
                return Err(InternalError::configuration_error(
                    &format!("Invalid data type for number field: {}", key),
                    None,
                ));
            }
        }

        if let Some(FieldDefault { value, .. }) = default {
            let default_num = value
                .as_deref()
                .unwrap_or("0")
                .parse::<f64>()
                .map(serde_json::Number::from_f64);

            match default_num {
                Ok(Some(num)) => return Ok(Value::Number(num)),
                _ => {
                    return Err(InternalError::configuration_error(
                        &format!("Invalid default value for number field: {}", key),
                        None,
                    ));
                }
            }
        }

        if required {
            return Err(InternalError::configuration_error(
                &format!("Missing required field to decode as number: {}", key),
                None,
            ));
        }

        Ok(Value::Null)
    }

    fn object(
        data: &Value,
        key: &str,
        fields: &HashMap<String, Field>,
        required: bool,
    ) -> Result<Value, IntegrationOSError> {
        let obj = map_data_by_schema(data, fields)?;

        match obj {
            Value::Object(obj) if !obj.is_empty() || !required => Ok(Value::Object(obj)),
            Value::Object(_) if required => Err(InternalError::configuration_error(
                &format!("Missing required field to decode as object: {}", key),
                None,
            )),
            _ => Err(InternalError::configuration_error(
                &format!("Invalid data type for object field: {}", key),
                None,
            )),
        }
    }

    fn array(
        data: &Value,
        key: &str,
        path: &str,
        items: &Field,
    ) -> Result<Value, IntegrationOSError> {
        fn go(
            acc: &mut Vec<Value>,
            key: &str,
            items: &Field,
            rest: &[Value],
        ) -> Result<Vec<Value>, IntegrationOSError> {
            if rest.is_empty() {
                return Ok(acc.clone());
            }

            acc.push(get_field_value(items, &rest[0], key)?);

            go(acc, key, items, &rest[1..])
        }

        let acc = select(data, path)
            .unwrap_or_else(|_| vec![])
            .iter()
            .try_fold(vec![], |mut acc, value| {
                match value {
                    Value::Number(_) => {
                        acc.push(number(data, key, path, false, &None)?);
                    }
                    Value::Bool(_) => {
                        acc.push(boolean(data, key, path, false, &None)?);
                    }
                    Value::String(_) => {
                        acc.push(string(data, key, path, false, &None)?);
                    }
                    Value::Object(_) => {
                        let configuration = SchemaMappingDefinition::from_iter(
                            vec![(key.to_string(), items.clone())].into_iter(),
                        );

                        let value = match object(data, key, &configuration, false)?.get(key) {
                            Some(arr) => Ok::<_, IntegrationOSError>(arr.clone()),
                            _ => {
                                return Err(InternalError::configuration_error(
                                    &format!("Invalid data type for array field: {key}"),
                                    None,
                                ))
                            }
                        };

                        acc.push(value?);
                    }
                    Value::Array(rest) => {
                        acc.extend(go(&mut vec![], key, items, rest)?);
                    }
                    _ => {
                        return Err(InternalError::configuration_error(
                            &format!("Invalid data type for array field: {key}"),
                            None,
                        ))
                    }
                };
                Ok(acc)
            })?;

        Ok(Value::Array(acc))
    }

    fn get_field_value(
        field: &Field,
        data: &Value,
        key: &str,
    ) -> Result<Value, IntegrationOSError> {
        match field {
            Field::String {
                required,
                default,
                path,
                ..
            } => string(data, key, path, *required, default),
            Field::Boolean {
                required,
                default,
                path,
                ..
            } => boolean(data, key, path, *required, default),
            Field::Number {
                required,
                default,
                path,
                ..
            } => number(data, key, path, *required, default),
            Field::Object {
                fields, required, ..
            } => object(data, key, fields, *required),
            Field::Array { items, path, .. } => array(data, key, path, items),
        }
    }

    let result: serde_json::Map<String, Value> =
        config
            .iter()
            .try_fold(serde_json::Map::new(), |mut acc, (key, field)| {
                acc.insert(key.clone(), get_field_value(field, data, key)?);
                Ok::<_, IntegrationOSError>(acc)
            })?;

    Ok(Value::Object(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_json_mapper_simple_config() {
        // Define a simple configuration
        let mut config = SchemaMappingDefinition::new();
        config.insert(
            "id".to_string(),
            Field::String {
                path: "$.data.id".to_string(),
                transformation: "identity".to_string(),
                required: true,
                default: None,
            },
        );
        config.insert(
            "name".to_string(),
            Field::String {
                path: "$.data.name".to_string(),
                transformation: "identity".to_string(),
                required: true,
                default: None,
            },
        );

        // Define input data
        let data = json!({
            "data": {
                "id": "123",
                "name": "Test",
            }
        });

        // Call the function
        let result = map_data_by_schema(&data, &config);

        // Define expected output
        let expected = json!({
            "id": "123",
            "name": "Test",
        });

        // Assert that the output matches the expected result
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_json_mapper_simple_config_with_default() {
        // Define a complex configuration as a JSON string
        let config_json = r#"
        {
            "id": {
                "type": "string",
                "path": "$.data.id",
                "transformation": "identity",
                "required": true
            },
            "name": {
                "type": "string",
                "path": "$.data.name",
                "transformation": "identity",
                "required": true
            },
            "orders": {
                "type": "array",
                "items": {
                    "type": "string",
                    "required": true,
                    "path": "$.data.name",
                    "transformation": "identity"
                },
                "transformation": "identity",
                "required": true,
                "path": "$"
            },
            "products_list": {
                "type": "array",
                "required": false,
                "path": "$.data.products",
                "items": {
                    "type": "object",
                    "required": true,
                    "fields": {
                        "id": {
                            "type": "string",
                            "path": "$.id",
                            "transformation": "identity",
                            "required": true
                        },
                        "name": {
                            "type": "string",
                            "path": "$.name",
                            "transformation": "identity",
                            "required": true
                        },
                        "value": {
                            "type": "number",
                            "path": "$.value",
                            "transformation": "identity",
                            "required": false,
                            "default": {
                                "value": "0"
                            }
                        }
                    }
                }
            },
            "product_names": {
                "type": "array",
                "path": "$.data.products",
                "transformation": "identity",
                "required": true,
                "items": {
                    "type": "string",
                    "path": "$.name",
                    "required": true,
                    "transformation": "identity"
                }
            }
        }
        "#;

        // Parse the configuration
        let config: SchemaMappingDefinition = serde_json::from_str(config_json).unwrap();

        // Define input data
        let data = json!({
            "data": {
                "id": "123",
                "name": "Test",
                "author": {
                    "id": "456",
                    "email": "test@example.com"
                },
                "products": [
                    {
                        "id": "789",
                        "name": "Product1"
                    },
                    {
                        "id": "012",
                        "name": "Product2",
                        "value": 100
                    }
                ]
            }
        });

        // Call the function
        let result = map_data_by_schema(&data, &config);

        // Define expected output
        let expected = json!({
            "id": "123",
            "product_names": [
              "Product1",
              "Product2"
            ],
            "orders": [
              "Test"
            ],
            "name": "Test",
            "products_list": [
              {
                "name": "Product1",
                "value": 0.0,
                "id": "789"
              },
              {
                "name": "Product2",
                "value": 100,
                "id": "012"
              }]
        });

        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_json_mapper_complex_config() {
        // Define a complex configuration as a JSON string
        let config_json = r#"
        {
            "Products": {
              "type": "object",
              "required": true,
              "fields": {
                "id": {
                  "type": "string",
                  "path": "$.id",
                  "transformation": "identity",
                  "required": false
                },
                "name": {
                  "type": "string",
                  "path": "$.name",
                  "transformation": "identity",
                  "required": false
                },
                "description": {
                  "type": "string",
                  "path": "$.description",
                  "transformation": "identity",
                  "required": false
                },
                "sku": {
                  "type": "string",
                  "path": "$.metadata.Sku",
                  "transformation": "identity",
                  "required": false
                },
                "price": {
                  "type": "number",
                  "path": "$.default_price",
                  "transformation": "identity",
                  "required": false
                },
                "quantity": {
                  "type": "number",
                  "path": "$.stock",
                  "transformation": "identity",
                  "required": false
                },
                "createdDate": {
                  "type": "string",
                  "path": "$.created",
                  "transformation": "identity",
                  "required": false
                },
                "updatedDate": {
                  "type": "string",
                  "path": "$.updated",
                  "transformation": "identity",
                  "required": false
                }
              }
            },
            "Attachments": {
              "type": "object",
              "required": true,
              "fields": {
                "url": {
                  "type": "string",
                  "path": "$.images",
                  "transformation": "identity",
                  "required": false
                }
              }
            }
          }
        "#;

        // Parse the configuration
        let config: SchemaMappingDefinition = serde_json::from_str(config_json).unwrap();

        // Define input data
        let data = json!({
          "id": "prod_OQWcqRkPyVteec",
          "object": "product",
          "active": true,
          "created": 1691699684,
          "default_price": null,
          "description": null,
          "images": [],
          "livemode": false,
          "metadata": {
            "ItemName": "63ea0249-76b0-4387-b25d-619758971594",
            "ItemId": "63ea0249-76b0-4387-b25d-619758971594",
            "Sku": "SUB-MB-S-030",
            "Duration": "30",
            "Service_SKU": "WP-SHINE"
          },
          "name": "Good 30 Day Unlimited Membership Subscription",
          "package_dimensions": null,
          "shippable": null,
          "statement_descriptor": null,
          "tax_code": null,
          "unit_label": null,
          "updated": 1691699684,
          "url": null
        });

        // Call the function
        let result = map_data_by_schema(&data, &config);

        // Define expected output
        let expected = json!({
          "Products": {
            "createdDate": 1691699684,
            "description": null,
            "id": "prod_OQWcqRkPyVteec",
            "name": "Good 30 Day Unlimited Membership Subscription",
            "price": null,
            "quantity": null,
            "sku": "SUB-MB-S-030",
            "updatedDate": 1691699684
          },
          "Attachments": {
            "url": []
          }
        });

        // Assert that the output matches the expected result
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_json_mapper_complex_config_with_array() {
        // Define a complex configuration as a JSON string
        let config_json = r#"
        {
            "Invoices": {
              "type": "object",
              "required": true,
              "fields": {
                "id": {
                  "type": "string",
                  "path": "$.id",
                  "transformation": "identity",
                  "required": false
                },
                "name": {
                  "type": "string",
                  "path": "$.object",
                  "transformation": "identity",
                  "required": true
                },
                "createdDate": {
                  "type": "string",
                  "path": "$.created",
                  "transformation": "identity",
                  "required": true
                },
                "updatedDate": {
                  "type": "string",
                  "path": "$.updatedDate",
                  "transformation": "identity",
                  "required": false
                },
                "dashboard": {
                  "type": "string",
                  "path": "$.livemode",
                  "transformation": "identity",
                  "required": false
                },
                "issueDate": {
                  "type": "string",
                  "path": "$.period_start",
                  "transformation": "identity",
                  "required": true
                },
                "dueDate": {
                  "type": "string",
                  "path": "$.due_date",
                  "transformation": "identity",
                  "required": false
                },
                "totalAmount": {
                  "type": "string",
                  "path": "$.total",
                  "transformation": "identity",
                  "required": true
                },
                "totalAmount2": {
                  "type": "string",
                  "path": "$.total",
                  "transformation": "identity",
                  "required": true
                },
                "currency": {
                  "type": "string",
                  "path": "$.currency",
                  "transformation": "identity",
                  "required": true
                },
                "status": {
                  "type": "string",
                  "path": "$.status",
                  "transformation": "identity",
                  "required": true
                }
              }
            },
            "People": {
              "type": "object",
              "required": true,
              "fields": {
                "id": {
                  "type": "string",
                  "path": "$.customer",
                  "transformation": "identity",
                  "required": false
                }
              }
            }
          }
        "#;

        // Parse the configuration
        let config: SchemaMappingDefinition = serde_json::from_str(config_json).unwrap();

        // Define input data
        let data = serde_json::from_str(
            r#"{
          "id": "in_1NdDbo2eZvKYlo2ChVkjULNO",
          "object": "invoice",
          "account_country": "US",
          "account_name": "Stripe.com",
          "account_tax_ids": null,
          "amount_due": 6525,
          "amount_paid": 0,
          "amount_remaining": 6525,
          "amount_shipping": 0,
          "application": null,
          "application_fee_amount": null,
          "attempt_count": 0,
          "attempted": false,
          "auto_advance": false,
          "automatic_tax": {
            "enabled": false,
            "status": null
          },
          "billing_reason": "manual",
          "charge": null,
          "collection_method": "charge_automatically",
          "created": 1691592216,
          "currency": "usd",
          "custom_fields": null,
          "customer": "cus_9s6XKzkNRiz8i3",
          "customer_address": null,
          "customer_email": null,
          "customer_name": null,
          "customer_phone": null,
          "customer_shipping": null,
          "customer_tax_exempt": "none",
          "customer_tax_ids": [],
          "default_payment_method": null,
          "default_source": null,
          "default_tax_rates": [],
          "description": null,
          "discount": null,
          "discounts": [],
          "due_date": null,
          "effective_at": null,
          "ending_balance": null,
          "footer": null,
          "from_invoice": null,
          "hosted_invoice_url": null,
          "invoice_pdf": null,
          "last_finalization_error": null,
          "latest_revision": null,
          "lines": {
            "object": "list",
            "data": [
              {
                "id": "il_1NdDbo2eZvKYlo2CkcXNP8j8",
                "object": "line_item",
                "amount": 6525,
                "amount_excluding_tax": 6525,
                "currency": "usd",
                "description": "My First Invoice Item (created for API docs)",
                "discount_amounts": [],
                "discountable": true,
                "discounts": [],
                "invoice_item": "ii_1NdDbo2eZvKYlo2C2n5wbKAJ",
                "livemode": false,
                "metadata": {},
                "period": {
                  "end": 1691592216,
                  "start": 1691592216
                },
                "price": {
                  "id": "price_1NdCuR2eZvKYlo2C1tj5f4eK",
                  "object": "price",
                  "active": true,
                  "billing_scheme": "per_unit",
                  "created": 1691589527,
                  "currency": "usd",
                  "custom_unit_amount": null,
                  "livemode": false,
                  "lookup_key": null,
                  "metadata": {},
                  "nickname": null,
                  "product": "prod_OQ30HMjKSXi1Vt",
                  "recurring": null,
                  "tax_behavior": "unspecified",
                  "tiers_mode": null,
                  "transform_quantity": null,
                  "type": "one_time",
                  "unit_amount": 6525,
                  "unit_amount_decimal": "6525"
                },
                "proration": false,
                "proration_details": {
                  "credited_items": null
                },
                "quantity": 1,
                "subscription": null,
                "tax_amounts": [],
                "tax_rates": [],
                "type": "invoiceitem",
                "unit_amount_excluding_tax": "6525"
              }
            ],
            "has_more": false,
            "url": "/v1/invoices/in_1NdDbo2eZvKYlo2ChVkjULNO/lines"
          },
          "livemode": false,
          "metadata": {},
          "next_payment_attempt": null,
          "number": null,
          "on_behalf_of": null,
          "paid": false,
          "paid_out_of_band": false,
          "payment_intent": null,
          "payment_settings": {
            "default_mandate": null,
            "payment_method_options": null,
            "payment_method_types": null
          },
          "period_end": 1688482163,
          "period_start": 1688395763,
          "post_payment_credit_notes_amount": 0,
          "pre_payment_credit_notes_amount": 0,
          "quote": null,
          "receipt_number": null,
          "redaction": null,
          "rendering_options": null,
          "shipping_cost": null,
          "shipping_details": null,
          "starting_balance": 0,
          "statement_descriptor": null,
          "status": "draft",
          "status_transitions": {
            "finalized_at": null,
            "marked_uncollectible_at": null,
            "paid_at": null,
            "voided_at": null
          },
          "subscription": null,
          "subscription_details": {
            "metadata": null
          },
          "subtotal": 6525,
          "subtotal_excluding_tax": 6525,
          "tax": null,
          "test_clock": null,
          "total": 6525,
          "total_discount_amounts": [],
          "total_excluding_tax": 6525,
          "total_tax_amounts": [],
          "transfer_data": null,
          "webhooks_delivered_at": null
        }"#,
        )
        .unwrap();

        // Call the function
        let result = map_data_by_schema(&data, &config);

        // Define expected output
        let expected = json!({
          "Invoices": {
            "createdDate": 1691592216,
            "currency": "usd",
            "dashboard": false,
            "dueDate": null,
            "id": "in_1NdDbo2eZvKYlo2ChVkjULNO",
            "issueDate": 1688395763,
            "name": "invoice",
            "status": "draft",
            "totalAmount": 6525,
            "totalAmount2": 6525,
            "updatedDate": null
          },
          "People": {
            "id": "cus_9s6XKzkNRiz8i3"
          }
        });

        // Assert that the output matches the expected result
        assert_eq!(result.unwrap(), expected);
    }

    #[ignore]
    #[test]
    fn test_json_mapper_complex_config_with_filters() {
        // TODO: Add tests for fields with filters such as [*].name, see
        // https://goessner.net/articles/JsonPath/ which is not supported
        // by the current implementation
    }
}
