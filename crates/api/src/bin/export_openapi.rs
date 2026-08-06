//! Export the CASIROS `OpenAPI` contract as pretty-printed JSON.
//!
//! Run with:
//!
//! ```bash
//! cargo run -p casiros-api --bin casiros-api-export-openapi > casiros.openapi.json
//! ```

#![forbid(unsafe_code)]

use casiros_api::openapi;

fn main() {
    print!("{}", openapi::spec_pretty());
}
