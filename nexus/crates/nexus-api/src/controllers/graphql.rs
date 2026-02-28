//! GraphQL endpoint — dynamically generates a schema from the SchemaOverview.
//! Uses async-graphql v7 dynamic schema API.

use crate::app_state::AppState;
use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use async_graphql::dynamic::{
    Field, FieldFuture, FieldValue, InputValue, Object, Schema, TypeRef,
};
use async_graphql::Value as GqlValue;
use nexus_services::items::ItemsService;
use nexus_types::accountability::Accountability;
use nexus_types::fields::FieldType;
use nexus_types::query::Query;
use nexus_types::schema::{CollectionOverview, SchemaOverview};
use serde_json::Value;

/// Build a dynamic GraphQL schema from the current SchemaOverview.
pub fn build_schema(schema_overview: &SchemaOverview, app_state: web::Data<AppState>) -> Schema {
    let mut query = Object::new("Query");

    for (coll_name, _coll) in &schema_overview.collections {
        let coll_type_name = to_pascal_case(coll_name);

        // List query: e.g., articles(limit: Int, offset: Int, sort: [String!]): [Article!]!
        let coll_name_owned = coll_name.clone();
        let app = app_state.clone();
        let list_field = Field::new(
            coll_name.clone(),
            TypeRef::named_nn_list_nn(&coll_type_name),
            move |ctx| {
                let coll = coll_name_owned.clone();
                let app = app.clone();
                FieldFuture::new(async move {
                    let accountability = ctx
                        .ctx
                        .data_opt::<Option<Accountability>>()
                        .cloned()
                        .flatten();
                    let svc_ctx = app.service_context(accountability).await;
                    let service = ItemsService::new(&coll, svc_ctx);

                    let limit = ctx.args.try_get("limit").ok().and_then(|v| v.i64().ok());
                    let offset = ctx
                        .args
                        .try_get("offset")
                        .ok()
                        .and_then(|v| v.i64().ok());
                    let sort = ctx.args.try_get("sort").ok().and_then(|v| {
                        v.list().ok().map(|list| {
                            list.iter()
                                .filter_map(|item| item.string().ok().map(|s| s.to_string()))
                                .collect::<Vec<_>>()
                        })
                    });

                    let query = Query {
                        limit,
                        offset,
                        sort,
                        ..Default::default()
                    };

                    let items = service
                        .read_by_query(query, None)
                        .await
                        .map_err(|e| async_graphql::Error::new(e.to_string()))?;

                    let values: Vec<FieldValue> =
                        items.into_iter().map(json_to_field_value).collect();

                    Ok(Some(FieldValue::list(values)))
                })
            },
        )
        .argument(InputValue::new("limit", TypeRef::named(TypeRef::INT)))
        .argument(InputValue::new("offset", TypeRef::named(TypeRef::INT)))
        .argument(InputValue::new(
            "sort",
            TypeRef::named_nn_list(TypeRef::STRING),
        ));

        query = query.field(list_field);

        // Single-item query: e.g., articles_by_id(id: ID!): Article
        let coll_name_owned2 = coll_name.clone();
        let app2 = app_state.clone();
        let by_id_field = Field::new(
            format!("{}_by_id", coll_name),
            TypeRef::named(&coll_type_name),
            move |ctx| {
                let coll = coll_name_owned2.clone();
                let app = app2.clone();
                FieldFuture::new(async move {
                    let id_accessor = ctx
                        .args
                        .try_get("id")
                        .map_err(|_| async_graphql::Error::new("Missing id argument"))?;

                    let id_str = id_accessor
                        .string()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|_| {
                            id_accessor
                                .i64()
                                .map(|n| n.to_string())
                                .unwrap_or_default()
                        });

                    let pk_value = nexus_types::items::PrimaryKey::String(id_str);
                    let accountability = ctx
                        .ctx
                        .data_opt::<Option<Accountability>>()
                        .cloned()
                        .flatten();
                    let svc_ctx = app.service_context(accountability).await;
                    let service = ItemsService::new(&coll, svc_ctx);

                    let item = service
                        .read_one(&pk_value, None, None)
                        .await
                        .map_err(|e| async_graphql::Error::new(e.to_string()))?;

                    Ok(Some(json_to_field_value(item)))
                })
            },
        )
        .argument(InputValue::new("id", TypeRef::named_nn(TypeRef::ID)));

        query = query.field(by_id_field);
    }

    // Build the schema
    let mut builder = Schema::build("Query", None, None).register(query);

    // Register all collection types
    for (coll_name, coll) in &schema_overview.collections {
        let item_type = build_collection_type(coll_name, coll);
        builder = builder.register(item_type);
    }

    builder.finish().expect("Failed to build GraphQL schema")
}

/// Build an async-graphql Object type for a collection.
fn build_collection_type(name: &str, coll: &CollectionOverview) -> Object {
    let type_name = to_pascal_case(name);
    let mut obj = Object::new(&type_name);

    for (field_name, field_overview) in &coll.fields {
        let gql_type = field_type_to_gql(&field_overview.field_type);
        let field_name_owned = field_name.clone();

        let field = Field::new(field_name.clone(), gql_type, move |ctx| {
            let field_name = field_name_owned.clone();
            FieldFuture::new(async move {
                let parent = ctx.parent_value.try_downcast_ref::<Value>()?;
                let val = parent.get(&field_name).cloned().unwrap_or(Value::Null);
                Ok(Some(json_to_field_value(val)))
            })
        });

        obj = obj.field(field);
    }

    obj
}

/// Map a Nexus FieldType to a GraphQL TypeRef.
fn field_type_to_gql(ft: &FieldType) -> TypeRef {
    match ft {
        FieldType::String | FieldType::Text | FieldType::Uuid | FieldType::Hash | FieldType::Csv => {
            TypeRef::named(TypeRef::STRING)
        }
        FieldType::Integer | FieldType::BigInteger => TypeRef::named(TypeRef::INT),
        FieldType::Float | FieldType::Decimal => TypeRef::named(TypeRef::FLOAT),
        FieldType::Boolean => TypeRef::named(TypeRef::BOOLEAN),
        FieldType::DateTime | FieldType::Date | FieldType::Time | FieldType::Timestamp => {
            TypeRef::named(TypeRef::STRING)
        }
        FieldType::Json => TypeRef::named(TypeRef::STRING),
        _ => TypeRef::named(TypeRef::STRING),
    }
}

/// Convert a serde_json::Value to an async-graphql FieldValue.
fn json_to_field_value(val: Value) -> FieldValue<'static> {
    match val {
        Value::Null => FieldValue::NULL,
        Value::Bool(b) => FieldValue::value(GqlValue::Boolean(b)),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                FieldValue::value(GqlValue::Number(i.into()))
            } else if let Some(f) = n.as_f64() {
                FieldValue::value(GqlValue::Number(
                    async_graphql::Number::from_f64(f).unwrap_or_else(|| 0.into()),
                ))
            } else {
                FieldValue::value(GqlValue::String(n.to_string()))
            }
        }
        Value::String(s) => FieldValue::value(GqlValue::String(s)),
        Value::Array(arr) => {
            let values: Vec<FieldValue> = arr.into_iter().map(json_to_field_value).collect();
            FieldValue::list(values)
        }
        Value::Object(_) => FieldValue::owned_any(val),
    }
}

/// Convert "snake_case" or "kebab-case" to "PascalCase" for GraphQL type names.
fn to_pascal_case(s: &str) -> String {
    s.split(|c: char| c == '_' || c == '-')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => {
                    let mut s = c.to_uppercase().to_string();
                    s.push_str(&chars.collect::<String>());
                    s
                }
            }
        })
        .collect()
}

/// POST /graphql — handle GraphQL queries
pub async fn graphql_handler(
    req: HttpRequest,
    app_state: web::Data<AppState>,
    body: web::Json<async_graphql::Request>,
) -> HttpResponse {
    let schema_overview = app_state.schema.read().await.clone();
    let schema = build_schema(&schema_overview, app_state.clone());

    // Extract accountability from request extensions (set by auth middleware)
    let accountability = req.extensions().get::<Accountability>().cloned();

    let request = body.into_inner().data(accountability);
    let response = schema.execute(request).await;

    HttpResponse::Ok().json(response)
}

/// Configure GraphQL routes
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("", web::post().to(graphql_handler));
}
