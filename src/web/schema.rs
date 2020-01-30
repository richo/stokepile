#![allow(proc_macro_derive_resolution_fallback)]

table! {
    components (id) {
        id -> Int4,
        equipment_id -> Int4,
        kind -> Varchar,
        manufacturer -> Varchar,
        model -> Varchar,
        serial -> Varchar,
        manufactured -> Date,
        data -> Jsonb,
    }
}

table! {
    customers (id) {
        id -> Int4,
        user_id -> Int4,
        name -> Varchar,
        address -> Varchar,
        phone_number -> Varchar,
        email -> Varchar,
    }
}

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
    equipment (id) {
        id -> Int4,
        user_id -> Int4,
        customer_id -> Int4,
    }
}

table! {
    global_settings (onerow_id) {
        onerow_id -> Bool,
        invites_required -> Bool,
    }
}

table! {
    integrations (id) {
        id -> Int4,
        user_id -> Int4,
        provider -> Text,
        access_token -> Text,
        refresh_token -> Nullable<Text>,
        refreshed -> Timestamp,
    }
}

table! {
    invites (id) {
        id -> Int4,
        email -> Varchar,
        consumed -> Nullable<Timestamp>,
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
    repacks (id) {
        id -> Int4,
        rigger -> Int4,
        equipment -> Int4,
        date -> Date,
        service -> Varchar,
        location -> Varchar,
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
        staging_data -> Nullable<Varchar>,
        preserve_device_files -> Bool,
        admin -> Bool,
    }
}

joinable!(components -> equipment (equipment_id));
joinable!(customers -> users (user_id));
joinable!(devices -> users (user_id));
joinable!(equipment -> customers (customer_id));
joinable!(equipment -> users (user_id));
joinable!(integrations -> users (user_id));
joinable!(keys -> users (user_id));
joinable!(repacks -> equipment (equipment));
joinable!(repacks -> users (rigger));
joinable!(sessions -> users (user_id));

allow_tables_to_appear_in_same_query!(
    components,
    customers,
    devices,
    equipment,
    global_settings,
    integrations,
    invites,
    keys,
    repacks,
    sessions,
    users,
);
