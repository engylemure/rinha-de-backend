mod models;
mod utils;
pub mod rinha {
    tonic::include_proto!("rinha");
    pub(crate) const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("rinha_descriptor");
}

#[cfg(feature = "without_cache_and_batch")]
mod without_cache;
#[cfg(feature = "without_cache_and_batch")]
pub use without_cache::*;

#[cfg(not(feature = "without_cache_and_batch"))]
mod with_cache;
#[cfg(not(feature = "without_cache_and_batch"))]
pub use with_cache::*;
