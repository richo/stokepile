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
        id -> Varchar,
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
    users (id) {
        id -> Int4,
        email -> Varchar,
        password -> Varchar,
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
