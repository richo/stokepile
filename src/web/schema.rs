#![allow(proc_macro_derive_resolution_fallback)]

table! {
    users (id) {
        id -> Int4,
        email -> Varchar,
        password -> Varchar,
    }
}
