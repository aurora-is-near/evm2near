// This is free and unencumbered software released into the public domain.

#![feature(stmt_expr_attributes)]

mod api;
mod env;
mod hash_provider;
mod near_runtime;
mod ops;
mod state;

#[cfg(test)]
mod ops_test;
