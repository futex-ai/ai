//! Reqwest OAuth transport with validated, DNS-pinned request hops.

mod request;
mod resolver;
mod response;
mod transport;

pub use resolver::SystemOAuthDnsResolver;
pub use transport::ReqwestOAuthHttpTransport;
