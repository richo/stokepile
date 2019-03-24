#![allow(proc_macro_derive_resolution_fallback)]

table! {
    devices (id) {
        id -> Int4,
        user_id -> Int4,
        name -> Varchar,
        kind -> Varchar,
        identifier -> Varchar,
    }
}

table! {
    integrations (id) {
        id -> Int4,
        user_id -> Int4,
        provider -> Text,
        access_token -> Text,
    }
}

table! {
    keys (id) {
        id -> Int4,
        user_id -> Int4,
        token -> Varchar,
        created -> Timestamp,
        expired -> Nullable<Timestamp>,
    }
}

table! {
    sessions (id) {
        id -> Varchar,
        user_id -> Int4,
        data -> Jsonb,
    }
}

table! {
    use diesel::sql_types::*;
    use crate::web::models::extra::StagingKindMapping;

    users (id) {
        id -> Int4,
        email -> Varchar,
        password -> Varchar,
        notify_email -> Nullable<Varchar>,
        notify_pushover -> Nullable<Varchar>,
        staging_type -> StagingKindMapping,
        staging_location -> Nullable<Varchar>,
    }
}

joinable!(devices -> users (user_id));
joinable!(integrations -> users (user_id));
joinable!(keys -> users (user_id));
joinable!(sessions -> users (user_id));

allow_tables_to_appear_in_same_query!(
    devices,
    integrations,
    keys,
    sessions,
    users,
);
