//! The channel contract lives in `botlib::traits::ChannelAdapter` (#1499);
//! this re-export keeps `crate::ChannelAdapter` resolving inside the crate.
pub use botlib::traits::ChannelAdapter;
