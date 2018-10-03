#![allow(proc_macro_derive_resolution_fallback)]

table! {
    integrations (id) {
        id -> Int4,
        user_id -> Int4,
        provider -> Text,
        access_token -> Text,
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

joinable!(integrations -> users (user_id));
joinable!(sessions -> users (user_id));

allow_tables_to_appear_in_same_query!(
    integrations,
    sessions,
    users,
);
