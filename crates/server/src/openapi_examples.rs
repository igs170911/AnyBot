use serde_json::Value;
use utoipa::Modify;
use utoipa::openapi::schema::{Object, Schema, SchemaType, Type};
use utoipa::openapi::{Components, OpenApi, RefOr};

const MAX_DEPTH: usize = 8;

pub(crate) struct ExampleFiller;

impl Modify for ExampleFiller {
    fn modify(&self, openapi: &mut OpenApi) {
        let Some(components) = &mut openapi.components else {
            return;
        };
        fill_components(components);
    }
}

fn fill_components(components: &mut Components) {
    for entry in components.schemas.values_mut() {
        if let RefOr::T(root) = entry {
            fill_schema(root, 0);
        }
    }
}

fn fill_schema(schema: &mut Schema, depth: usize) {
    if depth > MAX_DEPTH {
        return;
    }
    match schema {
        Schema::Object(object) => {
            fill_object(object);
            for prop in object.properties.values_mut() {
                fill_target(prop, depth.saturating_add(1));
            }
        }
        Schema::Array(array) => {
            if let utoipa::openapi::schema::ArrayItems::RefOrSchema(item) = &mut array.items {
                fill_target(item, depth.saturating_add(1));
            }
        }
        Schema::OneOf(composite) => {
            for item in &mut composite.items {
                fill_target(item, depth.saturating_add(1));
            }
        }
        Schema::AllOf(composite) => {
            for item in &mut composite.items {
                fill_target(item, depth.saturating_add(1));
            }
        }
        Schema::AnyOf(composite) => {
            for item in &mut composite.items {
                fill_target(item, depth.saturating_add(1));
            }
        }
        _ => {}
    }
}

fn fill_target(target: &mut RefOr<Schema>, depth: usize) {
    if let RefOr::T(schema) = target {
        fill_schema(schema, depth);
    }
}

fn fill_object(object: &mut Object) {
    if !is_scalar_type(&object.schema_type) {
        return;
    }
    if object.example.is_some() || !object.examples.is_empty() {
        return;
    }
    object.example = Some(example_value(object));
}

fn is_scalar_type(schema_type: &SchemaType) -> bool {
    match schema_type {
        SchemaType::Type(ty) => is_scalar(ty),
        SchemaType::Array(types) => types.iter().all(|ty| is_scalar(ty) || *ty == Type::Null),
        SchemaType::AnyValue => false,
    }
}

fn is_scalar(ty: &Type) -> bool {
    matches!(
        ty,
        Type::String | Type::Integer | Type::Number | Type::Boolean | Type::Null
    )
}

fn example_value(object: &Object) -> Value {
    if let Some(first) = object
        .enum_values
        .as_ref()
        .and_then(|values| values.first())
    {
        return first.clone();
    }
    match &object.schema_type {
        SchemaType::Type(Type::Boolean) => Value::Bool(false),
        SchemaType::Type(Type::Integer | Type::Number) => Value::from(0),
        SchemaType::Type(Type::Null) => Value::Null,
        SchemaType::Type(_) | SchemaType::Array(_) | SchemaType::AnyValue => {
            Value::String("sample".to_owned())
        }
    }
}
